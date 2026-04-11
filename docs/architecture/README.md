# Soliloquy Architecture Documentation

This directory contains architecture documentation for Soliloquy OS.

## Documents

### [System Architecture](./architecture.md)
High-level system architecture, component organization, and design principles.

**Key Topics**:
- Microkernel architecture (Zircon)
- Component model
- Inter-process communication
- System services
- Hardware abstraction

### [Component Manifests](./component_manifest.md)
Component structure, manifest files, and component configuration.

**Key Topics**:
- Component manifest format (`.cml`)
- Capability routing
- Component lifecycle
- Sandboxing and security
- Service declarations

### [Quick Reference](./quick_reference_manifest.md)
Quick reference guide for common commands and manifest patterns.

**Key Topics**:
- Common manifest patterns
- Command quick reference
- Troubleshooting tips

## Architecture Overview

```
┌─────────────────────────────────────────┐
│         User Applications               │
│  ┌──────────┬──────────┬──────────┐    │
│  │ Browser  │  Shell   │ Browser  │    │
│  │ Surface  │  APIs    │ Apps     │    │
│  └────┬─────┴──────────┴────┬─────┘    │
│       │ WebRender/WGPU      │          │
└───────┼─────────────────────┼───────────┘
        │                     │
┌───────▼─────────────────────▼───────────┐
│         Soliloquy Services              │
│  ┌──────────────────────────────────┐  │
│  │   Flatland (Compositor)          │  │
│  └──────────────────────────────────┘  │
│  ┌──────────────────────────────────┐  │
│  │   Component Manager              │  │
│  └──────────────────────────────────┘  │
└─────────────────┬───────────────────────┘
                  │ FIDL
┌─────────────────▼───────────────────────┐
│      Hardware Abstraction Layer         │
│  ┌──────┐ ┌──────┐ ┌────────┐          │
│  │ MMIO │ │ SDIO │ │Firmware│          │
│  └──────┘ └──────┘ └────────┘          │
└─────────────────┬───────────────────────┘
                  │
┌─────────────────▼───────────────────────┐
│   Drivers (WiFi, GPIO, Display, etc.)   │
└─────────────────┬───────────────────────┘
                  │
┌─────────────────▼───────────────────────┐
│        Zircon Microkernel               │
│  ┌──────────────────────────────────┐  │
│  │  Syscalls, IPC, Scheduling       │  │
│  └──────────────────────────────────┘  │
└─────────────────────────────────────────┘
```

## Key Concepts

### Microkernel Design
Soliloquy is built on Zircon, a capability-based microkernel:
- Minimal kernel (syscalls, IPC, memory management)
- System services run in user space
- Capability-based security model

### Component Model
Everything is a component:
- Isolated execution environments
- Capability routing for permissions
- Declarative manifests
- Dynamic lifecycle management

### FIDL (Fuchsia Interface Definition Language)
- Interface description language
- Cross-language IPC
- Type-safe communication
- Versioned protocols

### Web-Native Architecture
Soliloquy brings web technologies to system level:
- Servo for rendering
- V8 for scripting
- WebGPU/WGPU for graphics
- Web standards as first-class APIs

## Design Principles

1. **Capability-Based Security**
   - Explicit permission model
   - Least privilege principle
   - No ambient authority

2. **Modularity**
   - Components are independent
   - Clear interfaces (FIDL)
   - Pluggable implementations

3. **Web Standards**
   - HTML/CSS for UI
   - JavaScript for scripting
   - WebGPU for graphics
   - Standard web APIs

4. **Type Safety**
   - V language for kernel translations
   - Rust for system components
   - FIDL for IPC type safety

5. **Modern Hardware Support**
   - ARM64 first-class support
   - Mali GPU support
   - Modern peripherals (WiFi 6, etc.)

## Subsystems

### Shell (`src/shell/`)
Browser runtime and host bridge integrating Servo.

### Hardware Abstraction (`third_party/zircon_v/hal/`)
V translation of Zircon HAL providing hardware access primitives.

### Virtual Memory (`third_party/zircon_v/vm/`)
V translation of Zircon VM subsystem for memory management.

### IPC (`third_party/zircon_v/ipc/`)
Inter-process communication mechanisms.

### Drivers (`drivers/`)
Hardware drivers:
- WiFi (AIC8800D80)
- GPIO
- Display (Mali GPU)

### UI Framework (`ui/`)
UI components and Flatland integration for the single browser surface.

## Build System

Hybrid build system supporting multiple workflows:

- **GN + Ninja**: Full Fuchsia build (Linux)
- **Bazel**: Component-level builds (cross-platform)
- **Cargo**: Rust component builds

See [Build Guide](../build.md) for details.

## Translation Strategy

Gradual translation of Zircon from C to V:

1. **Phase 1**: HAL subsystem
2. **Phase 2**: VM subsystem
3. **Phase 3**: IPC subsystem
4. **Phase 4**: Scheduler and remaining subsystems

See [C2V Translation Guide](../translations/c2v_translations.md) for details.

## Target Hardware

**Primary Target**: Radxa Cubie A5E
- SoC: Allwinner A527 (ARM Cortex-A55)
- RAM: 8GB LPDDR4
- GPU: Mali-G57
- WiFi: AIC8800D80

## Performance Characteristics

- **Boot Time**: Target <10 seconds to shell
- **Memory Footprint**: ~256MB base system
- **Component Overhead**: <1MB per component
- **IPC Latency**: Target <100μs for local IPC

## Security Model

- Capability-based access control
- Component sandboxing
- No setuid/root model
- Explicit permission grants via manifests

## Development Model

- Component-based development
- FIDL-first design
- Test-driven development
- Continuous integration

## See Also

- [Developer Guide](../guides/dev_guide.md) - Development workflow
- [Build Guide](../build.md) - Build system documentation
- [Testing Guide](../testing/testing.md) - Testing strategies
- [C2V Translation](../translations/c2v_translations.md) - Translation guide
