# Soliloquy Architecture Documentation

This directory contains architecture documentation for Soliloquy OS.

## Documents

### [System Architecture](./architecture.md)
High-level system architecture, component organization, and design principles.

**Key Topics**:
- Alpine appliance architecture
- Servo desktop surface
- Local `sold` bridge APIs
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
│         Desktop Bundle                  │
│  ┌──────────┬──────────┬──────────┐    │
│  │ Browser  │  Shell   │ Terminal │    │
│  │ Surface  │  APIs    │ Surface  │    │
│  └────┬─────┴──────────┴────┬─────┘    │
│       │ WebRender/WGPU      │          │
└───────┼─────────────────────┼───────────┘
        │                     │
┌───────▼─────────────────────▼───────────┐
│         Soliloquy Services              │
│  ┌──────────────────────────────────┐  │
│  │   Servo Fullscreen Session       │  │
│  └──────────────────────────────────┘  │
│  ┌──────────────────────────────────┐  │
│  │   sold Local Bridge              │  │
│  └──────────────────────────────────┘  │
└─────────────────┬───────────────────────┘
                  │ Linux services
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
│        Alpine Linux Base                │
│  ┌──────────────────────────────────┐  │
│  │  Kernel, OpenRC, Filesystems     │  │
│  └──────────────────────────────────┘  │
└─────────────────────────────────────────┘
```

## Key Concepts

### Appliance Design
Soliloquy currently boots as an Alpine-based appliance:
- Immutable base image
- OpenRC-managed service startup
- Local system bridge for narrow privileged operations

### Service Model
System behavior is split across explicit services:
- Servo owns the fullscreen browser surface
- `sold` owns local system and terminal APIs
- Service policy is declared in system-owned configuration

### Local Bridge
- HTTP APIs are bound to local runtime origins
- Terminal and system calls require local bearer-token authentication
- Browser UI talks to `sold` instead of directly mutating the base image

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
   - Clear local bridge interfaces
   - Pluggable implementations

3. **Web Standards**
   - HTML/CSS for UI
   - JavaScript for scripting
   - WebGPU for graphics
   - Standard web APIs

4. **Type Safety**
   - Rust for browser runtime components
   - Typed `sold` API contracts
   - Explicit bridge schemas for Servo/RV8 boundaries

5. **Modern Hardware Support**
   - ARM64 first-class support
   - Mali GPU support
   - Modern peripherals (WiFi 6, etc.)

## Subsystems

### Shell (`src/shell/`)
Browser runtime and host bridge integrating Servo.

### Appliance System (`system/alpine/`)
Image staging, OpenRC service files, and runtime policy.

### Runtime Bridge (`sold/`)
Local bridge service for system APIs and terminal sessions.

### RV8 and Servo (`src/rv8/`, `third_party/servo/`)
Browser runtime and Servo/V8 linkage work.

### Drivers (`drivers/`)
Hardware drivers:
- WiFi (AIC8800D80)
- GPIO
- Display (Mali GPU)

### UI Framework (`ui/`)
UI components for the single browser surface.

## Build System

Hybrid build system supporting multiple workflows:

- **Alpine staging**: Full appliance image workflow
- **Bazel**: Component-level builds (cross-platform)
- **Cargo**: Rust component builds

See [Build Guide](../build.md) for details.

## Runtime Strategy

Servo boots the desktop appliance first while RV8 linkage moves toward real page-script ownership:

1. **Phase 1**: Alpine/OpenRC appliance boots Servo fullscreen
2. **Phase 2**: UI talks to `sold` over local authenticated APIs
3. **Phase 3**: Servo/RV8 bridge owns a bounded script and DOM command surface
4. **Phase 4**: RV8 replaces remaining fallback paths as bridge coverage expands

See [RV8 Linkage Roadmap](../rv8_linkage_roadmap.md) for details.

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

- Local bearer-token authentication for privileged bridge APIs
- Immutable base image with narrow writable state paths
- No browser-managed system writes
- Explicit service policy

## Development Model

- Service-based development
- Local API-first design
- Test-driven development
- Continuous integration

## See Also

- [Developer Guide](../guides/dev_guide.md) - Development workflow
- [Build Guide](../build.md) - Build system documentation
- [Testing Guide](../testing/testing.md) - Testing strategies
- [V0 Architecture](../v0-architecture.md) - Alpine appliance boot path
- [RV8 Linkage Roadmap](../rv8_linkage_roadmap.md) - Servo/RV8 bridge roadmap
