# Servo Integration Documentation

This document describes the integration of the Servo browser engine with V8 JavaScript runtime in Soliloquy.

## Overview

Soliloquy uses Servo as the primary UI rendering engine, combined with V8 for JavaScript execution. This integration provides:

- Web-based desktop environment
- Hardware-accelerated graphics via WebRender and Vulkan
- JavaScript execution with V8
- Touch and keyboard input handling
- Flatland compositor integration

## Architecture

```
┌─────────────────────────────────┐
│    Soliloquy Shell (Rust)       │
│  ┌──────────┐   ┌────────────┐  │
│  │  Servo   │   │ V8 Runtime │  │
│  └────┬─────┘   └────────────┘  │
│       │ WebRender/WGPU           │
│       ▼                          │
│  ┌──────────────────────────┐   │
│  │  Flatland (Compositor)   │   │
│  └──────────────────────────┘   │
└─────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────┐
│   Zircon Microkernel            │
└─────────────────────────────────┘
```

## Components

### ServoEmbedder

The main integration component (`src/shell/servo_embedder.rs`) that:

- Initializes V8 runtime
- Manages Servo webview instances
- Handles input events
- Manages Flatland graphics sessions
- Provides JavaScript execution context

### V8Runtime

A thin wrapper around rusty_v8 (`src/shell/v8_runtime.rs`) that:

- Creates and manages V8 isolates
- Provides JavaScript execution API
- Handles V8 platform initialization
- Manages memory and lifecycle

## Build Instructions

### Prerequisites

1. Local Servo checkout and native toolchain
2. Rust toolchain (1.75.0+)
3. Servo source code (cloned to vendor/servo)
4. rusty_v8 crate (via third_party/rust_crates)

### Setup

1. **Clone Servo as git submodule:**
   ```bash
   cd /path/to/soliloquy
   git submodule add https://github.com/servo/servo.git vendor/servo
   git submodule update --init --recursive
   ```

2. **Install dependencies:**
   ```bash
   ./tools/rv8_servo_test.sh bridge
   ```

3. **Build with GN:**
   ```bash
   # Set up Fuchsia environment
   source fuchsia/fuchsia/scripts/fx-env.sh
   
   # Build the shell
   fx build //vendor/soliloquy/src/shell:soliloquy_shell
   ```

4. **Build with Bazel:**
   ```bash
   bazel build //src/shell:soliloquy_shell
   ```

## Configuration

### Build Flags

The following build flags can be used to configure the Servo integration:

- `with_servo=true` - Enable Servo browser engine (default: true)
- `with_v8=true` - Enable V8 JavaScript runtime (default: true)
- `servo_debug=true` - Enable Servo debug logging (default: false)
- `v8_debug=true` - Enable V8 debug features (default: false)

### Runtime Configuration

The embedder can be configured through environment variables:

- `SERVO_LOG_LEVEL` - Servo logging level (debug, info, warn, error)
- `V8_FLAGS` - V8 command line flags
- `FLATLAND_WIDTH` - Default viewport width (default: 1920)
- `FLATLAND_HEIGHT` - Default viewport height (default: 1080)

## API Usage

### Basic Usage

```rust
use servo_embedder::ServoEmbedder;

// Create embedder
let mut embedder = ServoEmbedder::new()?;

// Load a URL
embedder.load_url("https://example.com")?;

// Execute JavaScript
let result = embedder.execute_js("document.title")?;
println!("Page title: {}", result);

// Handle input
embedder.handle_input(InputEvent::Touch { x: 100.0, y: 200.0 });

// Present frame
embedder.present()?;
```

### JavaScript Integration

The V8 runtime is automatically initialized and can execute JavaScript:

```rust
// Execute arbitrary JavaScript
let result = embedder.execute_js(r#"
    function calculateSum(a, b) {
        return a + b;
    }
    calculateSum(5, 3);
"#)?;

// Access DOM (when Servo is fully integrated)
let result = embedder.execute_js(r#"
    document.querySelector('h1').textContent;
"#)?;
```

### Input Handling

Input events are converted and forwarded to both Servo and JavaScript:

```rust
// Touch events
embedder.handle_input(InputEvent::Touch { x: 150.0, y: 250.0 });

// Key events
embedder.handle_input(InputEvent::Key { code: 13 }); // Enter
```

## Testing

### Unit Tests

Run unit tests for individual modules:

```bash
# Test V8 runtime
cargo test v8_runtime

# Test Servo embedder
cargo test servo_embedder
```

### Integration Tests

Run the full integration test suite:

```bash
# Run all integration tests
cargo test integration_tests

# Run specific test
cargo test test_complete_workflow
```

### Manual Testing

Build and run the shell to test manually:

```bash
# Build
fx build //vendor/soliloquy/src/shell:soliloquy_shell

# Run on device
fx run soliloquy_shell.cmx
```

## Troubleshooting

### Common Issues

1. **V8 Initialization Fails**
   - Ensure rusty_v8 is properly linked
   - Check that V8 binaries are available
   - Verify platform initialization

2. **Servo Loading Fails**
   - Check Servo submodule is initialized
   - Verify Servo build dependencies
   - Check for missing Servo libraries

3. **Flatland Connection Fails**
   - Ensure Flatland service is available
   - Check view reference creation
   - Verify graphics driver support

### Debug Logging

Enable debug logging to troubleshoot issues:

```bash
# Set log level
export RUST_LOG=debug

# Enable Servo debug
export SERVO_LOG_LEVEL=debug

# Enable V8 debug
export V8_FLAGS=--trace-gc
```

## Performance Considerations

### Memory Usage

- V8 isolate memory: ~50-100MB base
- Servo webview: ~20-50MB per page
- Flatland buffers: ~16-32MB for 1080p

### Optimization Tips

1. **JavaScript Performance**
   - Use `--optimize-for-size` V8 flag for memory-constrained devices
   - Enable V8 turbofan optimization
   - Limit concurrent script execution

2. **Graphics Performance**
   - Use appropriate Flatland buffer formats
   - Implement frame rate limiting
   - Optimize WebGL usage

3. **Memory Management**
   - Implement page lifecycle management
   - Use V8 heap size limits
   - Clean up unused resources

## Future Enhancements

### Planned Features

1. **Full Servo Integration**
   - Complete Servo API bindings
   - WebRender integration
   - WebGPU support

2. **Advanced JavaScript**
   - ES6+ features
   - WebAssembly support
   - Node.js compatibility layer

3. **Graphics Enhancements**
   - Multi-monitor support
   - Hardware video decoding
   - Advanced compositing

### Development Roadmap

- **Phase 1**: Basic V8 integration (✅ Complete)
- **Phase 2**: Servo webview management (✅ Complete)
- **Phase 3**: Flatland graphics integration (In Progress)
- **Phase 4**: Full web platform support (Planned)
- **Phase 5**: Performance optimization (Planned)

## Contributing

When contributing to the Servo integration:

1. Follow Rust coding standards
2. Add appropriate tests
3. Update documentation
4. Test on target hardware
5. Verify performance impact

## References

- [Servo Project](https://servo.org/)
- [V8 JavaScript Engine](https://v8.dev/)
- [Fuchsia Flatland](https://fuchsia.dev/fuchsia-src/development/graphics/flatland)
- [WebRender](https://github.com/servo/webrender)
- [rusty_v8](https://github.com/denoland/rusty_v8)
