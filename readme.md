# Soliloquy

A minimal, web-native operating system built on the Zircon microkernel with Servo as the desktop environment.

## Overview

Soliloquy is an experimental OS that brings web technologies to the system level. It uses:
- **Zircon** - Microkernel from Fuchsia
- **Servo + V8** - Desktop environment (when display available)
- **Svelte 5 UI** - Modern reactive frontend -> migrating to [Equilibrium (repo private for now)](https://github.com/atechnology-company/equilibrium)
- **WGPU** - Graphics via Vulkan

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines.

### Quick Contribution Guide

1. Fork and clone
2. Create feature branch
3. Make changes with tests
4. Run `bazel test //...`
5. Submit PR

## Documentation

📚 **[Complete Documentation Index](docs/INDEX.md)** - All documentation organized by topic

### Quick Links
- **[Getting Started Tutorial](docs/tutorials/getting_started.md)** - Complete setup guide for new developers
- **[Developer Guide](docs/guides/dev_guide.md)** - Development workflow and best practices
- **[Tools Reference](docs/guides/tools_reference.md)** - Complete tool documentation
- **[Build System Guide](docs/build.md)** - Building the project (GN, Bazel, Cargo)
- **[Architecture](docs/architecture/architecture.md)** - System design and components
- **[Testing Guide](docs/testing/testing.md)** - How to write and run tests
- **[C-to-V Translation](docs/translations/c2v_translations.md)** - Zircon subsystem translations
- **[Contributing](docs/contributing.md)** - Contribution guidelines

## Recent Updates

**Latest PRs:**
- PR #5: Servo desktop UI scaffold
- PR #4: macOS build stabilization
- PR #3: fx tooling hardening
- PR #2: V8 runtime integration
- PR #1: Soliloquy HAL + GPIO driver

## Acknowledgments

- Fuchsia Project - Zircon microkernel
- Servo Project - Browser engine
- V8 Project - JavaScript runtime
- Radxa - Hardware platform (really just the sbc i have)

---

**Note:** This is an experimental OS project. Not intended for production use.
