# Soliloquy Architecture

## Overview

Soliloquy is a minimal, web-native operating system built on the Zircon microkernel with Servo as a browser-only appliance surface.

## System Architecture

```
┌─────────────────────────────────────────────────────────┐
│                  User Applications                       │
│              (Web Apps via Servo)                        │
└─────────────────────────────────────────────────────────┘
                          │
┌─────────────────────────────────────────────────────────┐
│              Soliloquy Shell (Rust)                      │
│  ┌──────────────────┐         ┌────────────────────┐    │
│  │  Servo Engine    │◄───────►│  V8 Runtime        │    │
│  │  (Browser)       │         │  (JavaScript)      │    │
│  └────────┬─────────┘         └────────────────────┘    │
│           │                                              │
│           │ WebRender/WGPU                               │
│           ▼                                              │
│  ┌──────────────────────────────────────────────────┐   │
│  │        Flatland Compositor                        │   │
│  │        (Fuchsia UI)                               │   │
│  └──────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
                          │ FIDL
┌─────────────────────────────────────────────────────────┐
│              Fuchsia System Services                     │
│  ┌──────────┐  ┌──────────┐  ┌──────────────────────┐  │
│  │  Scenic  │  │  Input   │  │  Network Manager     │  │
│  └──────────┘  └──────────┘  └──────────────────────┘  │
└─────────────────────────────────────────────────────────┘
                          │
┌─────────────────────────────────────────────────────────┐
│              Soliloquy HAL                               │
│  ┌────────┐  ┌────────┐  ┌──────────┐  ┌────────────┐  │
│  │  MMIO  │  │  SDIO  │  │ Firmware │  │ Clock/RST  │  │
│  └────────┘  └────────┘  └──────────┘  └────────────┘  │
└─────────────────────────────────────────────────────────┘
                          │
┌─────────────────────────────────────────────────────────┐
│                    Device Drivers                        │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │  AIC8800     │  │  GPIO        │  │  Mali GPU    │  │
│  │  (WiFi)      │  │  (Allwinner) │  │  (Future)    │  │
│  └──────────────┘  └──────────────┘  └──────────────┘  │
└─────────────────────────────────────────────────────────┘
                          │
┌─────────────────────────────────────────────────────────┐
│              Zircon Microkernel                          │
│  (Process, Memory, IPC, Scheduling)                      │
└─────────────────────────────────────────────────────────┘
                          │
┌─────────────────────────────────────────────────────────┐
│              Hardware (Radxa Cubie A5E)                  │
│  Allwinner A527 (ARM Cortex-A55) + Mali-G57 GPU         │
└─────────────────────────────────────────────────────────┘
```

## Core Components

### 1. Soliloquy Shell
**Location:** `src/shell/`
**Language:** Rust
**Purpose:** Main browser runtime and application host

**Key Files:**
- `main.rs` - Entry point and initialization
- `servo_embedder.rs` - Servo browser integration
- `v8_runtime.rs` - JavaScript execution
- `zircon_window.rs` - Window management and Flatland integration

**Responsibilities:**
- Initialize Servo browser engine
- Manage V8 JavaScript runtime
- Handle user input (keyboard, mouse, touch)
- Render the single browser surface via Flatland compositor
- Coordinate between web content and system APIs

### 2. Servo Browser Engine
**Location:** `vendor/servo/`
**Language:** Rust
**Purpose:** Web rendering and layout

**Components:**
- **WebRender** - GPU-accelerated rendering
- **Layout Engine** - CSS layout and positioning
- **DOM** - Document object model
- **Networking** - HTTP/HTTPS requests

**Integration:**
- Embedded via `servo_embedder.rs`
- Renders to Flatland surfaces
- Executes JavaScript via V8
- Handles web standards (HTML, CSS, JavaScript)

### 3. V8 JavaScript Runtime
**Location:** Integrated via `rusty_v8`
**Language:** Rust bindings to C++
**Purpose:** Execute JavaScript code

**Features:**
- Script compilation and execution
- Isolate management
- Context creation
- Error handling

### 4. Soliloquy HAL
**Location:** `drivers/common/soliloquy_hal/`
**Language:** C++
**Purpose:** Hardware abstraction layer

**Modules:**
- **MMIO** - Memory-mapped I/O operations
- **SDIO** - SD/SDIO protocol implementation
- **Firmware** - Firmware loading utilities
- **Clock/Reset** - Clock gating and reset management

**Benefits:**
- Portable across hardware platforms
- Reusable across drivers
- Testable via mocking
- Clean separation of concerns

### 5. Device Drivers

#### WiFi Driver (AIC8800D80)
**Location:** `drivers/wifi/aic8800/`
**Language:** C++
**Status:** Scaffolding complete

**Features:**
- SDIO communication
- Firmware loading
- Network interface
- Power management

#### GPIO Driver
**Location:** `drivers/gpio/soliloquy_gpio/`
**Language:** C++
**Status:** Complete

**Features:**
- Pin configuration
- Digital I/O
- Interrupt handling (stub)
- Allwinner A527 specific

## Communication Patterns

### FIDL (Fuchsia Interface Definition Language)
Components communicate via FIDL protocols:

```
Shell Component ──FIDL──> Scenic (Compositor)
                ──FIDL──> Input Service
                ──FIDL──> Network Manager
```

**Key Protocols:**
- `fuchsia.ui.composition.Flatland` - Compositor
- `fuchsia.ui.views.ViewProvider` - View creation
- `fuchsia.input.*` - Input events

### Component Topology
```
soliloquy_shell.cm
├── uses: fuchsia.ui.composition.Flatland
├── uses: fuchsia.ui.views.ViewProvider
├── uses: fuchsia.input.keyboard
├── uses: fuchsia.input.mouse
└── uses: fuchsia.net.interfaces
```

## Data Flow

### Page Load Sequence
1. User enters URL in shell
2. Shell passes URL to Servo
3. Servo fetches content via network service
4. Servo parses HTML/CSS
5. Servo executes JavaScript via V8
6. Servo layouts content
7. WebRender generates GPU commands
8. Commands sent to Flatland
9. Flatland composites with other UI
10. GPU renders final frame

### Input Event Flow
1. Hardware generates interrupt
2. Kernel delivers to input service
3. Input service sends FIDL event
4. Shell receives event
5. Shell passes to Servo
6. Servo updates DOM
7. JavaScript handlers execute
8. UI updates rendered

## Build System

### Bazel
Primary build system for Soliloquy components.

**Structure:**
```
MODULE.bazel          # Bzlmod configuration
BUILD.bazel           # Root build file
src/shell/BUILD.bazel # Component builds
vendor/servo/BUILD.bazel # Servo integration
```

### GN (Future)
For full Fuchsia integration.

**Structure:**
```
BUILD.gn              # Root build file
src/shell/BUILD.gn    # Component builds
boards/arm64/soliloquy/BUILD.gn # Board config
```

## Security Model

### Capability-Based Security
Components only access services they declare:

```cml
{
  program: { runner: "elf" },
  use: [
    { protocol: "fuchsia.ui.composition.Flatland" },
    { protocol: "fuchsia.input.keyboard" },
  ]
}
```

### Sandboxing
- Each component runs in isolated process
- No ambient authority
- Explicit capability routing

## Performance Considerations

### GPU Acceleration
- WebRender uses Vulkan via Mali GPU
- Hardware-accelerated compositing
- Efficient memory management

### Memory Management
- Zircon VMOs for shared memory
- Copy-on-write for efficiency
- Explicit lifetime management

### Threading
- Servo uses parallel layout
- V8 isolates for concurrency
- Async I/O via Fuchsia async runtime

## Future Enhancements

### Planned Features
1. **Mali GPU Driver** - Full GPU acceleration
2. **Audio Support** - GStreamer integration
3. **Bluetooth** - Device connectivity
4. **Storage** - File system integration
5. **Multi-window** - Window management

### Research Areas
1. **WebAssembly** - Native performance for web apps
2. **Progressive Web Apps** - Offline capabilities
3. **WebGPU** - Advanced graphics
4. **WebXR** - AR/VR support

## References

- [Fuchsia Documentation](https://fuchsia.dev/)
- [Servo Book](https://book.servo.org/)
- [V8 Documentation](https://v8.dev/docs)
- [Zircon Kernel](https://fuchsia.dev/fuchsia-src/concepts/kernel)
