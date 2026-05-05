# Soliloquy Architecture

Soliloquy is a Servo + RV8/V8 desktop environment with integrated search, memory storage, and a local `sold` system bridge.

## Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    Soliloquy Desktop                         │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  Svelte 5 UI (ui/desktop)                             │  │
│  │  - SearchBar: Unified command/search/browser bar      │  │
│  │  - SearchCarousel: Perplexity-style result cards      │  │
│  │  - Local desktop shell + terminal                     │  │
│  └─────────────────┬─────────────────────────────────────┘  │
│                    │                                          │
│  ┌─────────────────▼─────────────────────────────────────┐  │
│  │  V Backend (backend/)                                  │  │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌─────────┐  │  │
│  │  │ search.v │ │cupboard.v│ │system.rs│ │ sold     │  │  │
│  │  │ (Unified)│ │(Memories)│ │ (APIs)  │ │(bridge)  │  │  │
│  │  └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬────┘  │  │
│  │       │            │            │            │         │  │
│  │       └────────────┴────────────┴────────────┘         │  │
│  │                    │                                    │  │
│  └────────────────────┼────────────────────────────────────┘  │
│                       │                                        │
└───────────────────────┼────────────────────────────────────────┘
                        │
        ┌───────────────┼───────────────┐
        │               │               │
        ▼               ▼               ▼
  ┌─────────┐    ┌──────────┐    ┌──────────────┐
  │ Plates  │    │Perplexity│    │ Alpine/sold  │
  │Tableware│    │   API    │    │  Services    │
  └─────────┘    └──────────┘    │ (Local)      │
                                  └──────────────┘
```

## Components

### UI Layer (`ui/desktop/`)

**Svelte 5 + Tailwind v4 + shadcn/ui**

- **SearchBar Component**: Large, minimal search input that acts as:
  - Command bar (Plates commands with `/` or `>`)
  - Search engine (web search via Perplexity)
  - Browser (URL navigation)
  - Memory retrieval (Cupboard queries)

- **SearchCarousel Component**: Horizontal scrolling card layout inspired by Perplexity
  - Card types: `web`, `cupboard`, `command`, `browser`
  - Smooth animations and hover effects
  - Click to navigate or execute

- **Authentication**: none for the desktop shell; `sold` handles the local terminal token

### Backend Layer (`backend/`)

**V Language + vweb framework**

#### `main.v`
- Entry point and vweb app setup
- Initializes Cupboard and local system bridge clients
- Health check endpoint

#### `search.v`
- Unified search interface
- Command parsing (`/command`, `>command`, URLs)
- Web search integration (Perplexity API)
- Cupboard memory search
- Returns carousel-ready `SearchCard[]`

#### `cupboard.v`
- Universal memory storage (inspired by Plates Cupboard)
- Stores: user memories, search history, clipboard, pickups
- Tag-based organization
- Vector embeddings (TODO)
- System-owned persistence through `sold`

#### `sold`
- Local bridge to system services
- Terminal session creation and I/O
- Policy, plugin, and service registry endpoints
- Services: terminal, policy, plugins, updates, service state

#### `tableware.v`
- Proxy to Plates Tableware MCP server
- Pickup session forwarding
- Onboarding state management

#### `config.v`
- Environment-based configuration
- Tableware endpoint
- Session secrets

## Data Flow

### Search Flow

1. User types in SearchBar
2. On submit → `POST /api/search` with query
3. Backend parses intent:
   - URL → return `browser` card
   - `/command` → return `command` card
   - Text → search Cupboard + Web
4. Returns `SearchResponse` with `SearchCard[]`
5. UI renders SearchCarousel
6. User clicks card → navigate/execute

### Memory Storage Flow

1. User interaction generates memory
2. `POST /api/cupboard/store` with content + metadata
3. Backend stores in-memory (dev) or through `sold` system-owned storage
4. Returns memory ID
5. Memory available for future retrieval via search

### Local Bridge Flow

1. `sold` starts as a local authenticated service
2. Frontend calls local API routes through the desktop runtime
3. Backend validates the bearer token and requested operation
4. System service state is read or updated through narrow endpoint handlers
5. UI receives typed JSON responses

## Integration Points

### Plates Tableware
- Pickup session sync
- Device status
- Clipboard history
- Real-time activity feed

### Perplexity API (TODO)
- Web search with AI summaries
- Carousel card generation
- Follow-up questions

### Local System Services
- Terminal: PTY-backed sessions
- Policy: system and browser storage policy
- Plugins: install and runtime state
- Updates: generation state and rollback metadata

## Security

- **Authentication**: none in the desktop shell; terminal/API calls use a local bearer token
- **CORS**: restricted to the local desktop/runtime origins
- **sold**: Local bridge APIs with token validation

## Performance

- **In-memory caching**: Sessions and memories cached in V maps
- **Lazy initialization**: System endpoints initialize only when requested
- **Conditional compilation**: Platform-specific code excluded via `$if`
- **Carousel rendering**: Virtual scrolling for large result sets (TODO)

## Headless Mode

When no display is detected, Soliloquy automatically runs as a **headless Cupboard sync server**:

- Servo + V8 desktop **does not start**
- Backend runs on port 3030 as a sync endpoint
- Devices can push/pull memories via `/api/sync/push` and `/api/sync/pull`
- Display detection: Uses local display/session availability from the appliance runtime

**Sync Endpoints**:
- `POST /api/sync/push` - Device pushes memories to server
- `POST /api/sync/pull` - Device pulls new memories from server
- `GET /api/sync/devices` - List registered devices
- `GET /api/sync/status` - Server mode and stats

**Use Cases**:
- Appliance device as headless Cupboard server (no monitor connected)
- Alpine-based SBC running as home sync hub
- Development boards for IoT data collection

## Development Workflow

### With Display (Desktop Mode)
1. Start Plates Tableware: `cd plates/tableware/backend && go run .`
2. Start Soliloquy: `./tools/soliloquy/start.sh`
3. Navigate to `http://localhost:5173`

### Headless Mode (Server Only)
1. Start Soliloquy backend: `cd backend && v run .`
2. Server available at `http://localhost:3030`
3. Connect devices to sync endpoint

**Quick Start Script**: `./tools/soliloquy/start.sh`
- Auto-detects display availability
- Starts backend + UI (if display present)
- Starts backend only (if headless)

## Deployment

1. Build the `sold` bridge with Cargo.
2. Build UI bundle: `cd ui/desktop && bun run build`
3. Stage the Alpine appliance artifacts.
4. Deploy to target device

## Future Enhancements

- [ ] Perplexity API integration for real web search
- [ ] Vector similarity search in Cupboard
- [ ] SurrealDB persistence layer
- [ ] WebRTC tunneling for remote access
- [ ] Native appliance service integrations (Audio, Display)
- [ ] Command palette with fuzzy search
- [ ] File browser integration
- [ ] Tab management
