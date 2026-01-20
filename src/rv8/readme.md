# RV8 Browser Engine

**Roverate V8** - A modern browser engine combining Servo's rendering with V8's JavaScript execution.

## Architecture

RV8 uses a Chrome-like multi-process architecture:

```
┌─────────────────────────────────────────────────────────┐
│                   Browser Process                       │
│  • Tab Management    • Navigation    • Process Control  │
├─────────────────────────────────────────────────────────┤
│                    IPC Channels                         │
├─────────────────────────────────────────────────────────┤
│  ┌──────────────────┐  ┌──────────────────────────────┐│
│  │ Renderer Process │  │           GPU Process        ││
│  │ (per tab)        │  │         (Compositor)         ││
│  │ • HTML/CSS Parse │  │  • Layer Compositing         ││
│  │ • Layout         │  │  • Hardware Acceleration     ││
│  │ • V8 JavaScript  │  └──────────────────────────────┘│
│  └──────────────────┘                                   │
│  ┌──────────────────────────────────────────────────┐  │
│  │              Network Process                      │  │
│  │         • HTTP/HTTPS • Caching • Cookies          │  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

## Features

- **V8 JavaScript Engine**: TurboFan + Sparkplug compilation
- **Servo Rendering**: WebRender-based GPU rendering
- **Chrome-like Optimizations**: Tab discarding, prefetching, code caching
- **Multi-Process**: Sandboxed renderers, site isolation
- **Modern Standards**: HTTP/3, Web APIs, DevTools Protocol

## Quick Start

```bash
# Build
cargo build -p rv8

# Run
cargo run -p rv8 -- https://example.com

# Run with single-process (debugging)
cargo run -p rv8 --features single-process -- https://example.com
```

## Structure

```
src/rv8/
├── lib.rs              # Library entry
├── main.rs             # Binary entry (multi-process)
├── core/               # Browser process
│   ├── browser.rs      # Main browser coordinator
│   ├── tab.rs          # Tab management
│   ├── config.rs       # Configuration
│   └── process_manager.rs # Child process spawning
├── renderer/           # Renderer process (Servo-based)
├── js/                 # JavaScript engine (V8)
├── compositor/         # GPU compositing
├── networking/         # Network stack
├── storage/            # Persistence (cookies, cache)
├── ipc/                # Inter-process communication
└── optimizations/      # Performance tuning
    ├── flags.rs        # Chrome-like optimization flags
    ├── monitor.rs      # Performance monitoring
    └── preload.rs      # Resource prefetching
```

## Development

See [TODO.md](./TODO.md) for the roadmap.

## License

MIT OR Apache-2.0