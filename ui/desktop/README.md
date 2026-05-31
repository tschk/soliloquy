# Soliloquy Servo Desktop UI

A Svelte v5 application that prototypes the Servo desktop surface for Soliloquy. The goal is to mirror the real runtime: Servo provides compositing, `sold` serves the local bundle, and this repo supplies the desktop shell authored in Svelte.

## Overview

- **Frontend:** Svelte 5 + Tailwind v4 + shadcn/ui components
- **Runtime contract:** Servo browser engine hosting the local desktop bundle and `sold`
- **Component Library:** shadcn-svelte for Button, Input, Label + reusable bits-ui primitives
- **Status:** Web build that Servo/V8 embed as the literal desktop surface (no more Tauri)

## Features

- **Ambient Greeting Surface:** Hero desktop panel with live time/date and glassmorphic glow.
- **Task Canvas:** Secondary glass panel showcasing "what can we get started" copy, quick filters, and featured pickups.
- **Command Palette:** Global command bar pinned bottom-right with ⌘/Ctrl + \ hotkey plus fuzzy suggestions.
- **Terminal Pane:** Browser terminal backed by the PTY bridge in `sold`.
- **Tableware Bridge:** Optional pickup/banner data from the sibling [`../plates/tableware`](../../plates/tableware) MCP service.
- **Responsive Layout:** Tailwind v4 + shadcn/ui components with custom Instrument Sans typography.

## Development

### Prerequisites

- Bun 1.3+
- Tailwind CSS v4 (installed via `@tailwindcss/vite`)
- optional V language backend (https://vlang.io) for richer search results

### Quick Start

1. **Start the optional Soliloquy backend (V):**

```bash
cd backend
v run main.v
```

The backend listens on `http://localhost:3030` and can enrich search results, but the desktop boots without it.

2. **Start the UI dev server:**

```bash
# From the project root
./tools/soliloquy/dev_ui.sh

# Or manually:
cd ui/desktop
bun install
bun run dev
```

### Scripts

- `bun run dev` – start the Svelte dev server (used by Servo during development)
- `bun run build` – build the static bundle that Servo hosts
- `bun run preview` – preview the production bundle locally
- `bun run check` – run `svelte-check` + type analysis
- `bun run check:watch` – watch mode for the same checks

### Tableware Endpoint

Set `VITE_TABLEWARE_BASE_URL` if your optional search service is not running on `http://localhost:3030`. The desktop will fall back to local web search cards if the backend is unavailable.

### Build Output

`bun run build` emits a static bundle in `build/`. `sold` serves that directory from `/usr/local/share/soliloquy/bundle` at runtime.

## Architecture

```
src/
├── routes/
│   ├── +layout.svelte      # Ambient surfaces + command button
│   ├── +page.svelte        # Desktop surface
│   └── dashboard/+page.svelte # Desktop surface alias
├── lib/
│   ├── components/
│   │   └── CommandPalette.svelte
│   ├── stores/
│   │   └── system.ts       # Clock + telemetry stores
│   └── system/
│       └── actions.ts      # Centralized system actions & command suggestions
├── app.css                 # Global styles + Instrument Sans import
└── app.html                # Template shell
```

### Command Palette

- `CommandPalette.svelte` exposes a suggestion list, input field, and close events
- Bound to the global shortcut (⌘/Ctrl + \) via layout-level listeners
- Suggestions are data-driven so Servo can later hydrate them from runtime metadata

### Routes

- `/` – Desktop shell and command bar
- `/dashboard` – Desktop shell alias

### State Management

- `systemClock` is a shared readable store updated once per second
- `clockDisplay` derives formatted strings via memoized `Intl.DateTimeFormat` instances (no repeated allocations)
- Components subscribe to the derived store to avoid duplicate intervals/timers

## Design System

- **Palette:** Deep black canvas with indigo/fuchsia glow orbs and soft glass panels
- **Typography:** Instrument Sans (400–700) for every heading, matched with uppercase tracking for system cues
- **Accessibility:** Command palette supports keyboard input, and the CTA button exposes clear labels + shortcuts

## Integration Path

1. **Prototype (current):** Pure web bundle served by Vite during development
2. **Servo Embed:** Servo loads the `sold`-served desktop scene
3. **Bridge:** `sold` exposes status, power, and PTY APIs for the desktop shell

## Contributing

1. Keep components declarative and side-effect free; use stores/utilities for shared logic
2. Prefer data-driven configuration so Servo can hydrate state from FIDL in the future
3. Run `bun run check` + `bun run build` before submitting changes
4. Document any new palette shortcuts or surface copy updates in this README
