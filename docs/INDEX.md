# Soliloquy OS Documentation Index

Welcome to the Soliloquy OS documentation! This index helps you find the information you need quickly.

## 📖 Getting Started

- **[README](../readme.md)** - Project overview and quick start
- **[Build Guide](./build.md)** - Building the project (GN, Bazel, Cargo)
- **[Contributing](./contributing.md)** - Contribution guidelines

## 📚 Guides

Comprehensive step-by-step guides for development tasks:

- **[Developer Guide](./guides/dev_guide.md)** - Complete development workflow
- **[Getting Started with Testing](./guides/getting_started_with_testing.md)** - How to run and write tests
- **[Driver Porting Guide](./guides/driver_porting.md)** - Porting drivers to Soliloquy
- **[Servo Integration Guide](./guides/servo_integration.md)** - Servo browser engine integration

## 🏗️ Architecture

System design and component structure documentation:

- **[System Architecture](./architecture/architecture.md)** - High-level system design
- **[Component Manifests](./architecture/component_manifest.md)** - Component structure and manifests
- **[Quick Reference](./architecture/quick_reference_manifest.md)** - Command quick reference

## 🧪 Testing

Testing strategies, frameworks, and best practices:

- **[Testing Guide](./testing/testing.md)** - Comprehensive testing documentation
- **[Test Coverage Expansion](./testing/test_coverage_broadening.md)** - Expanding test coverage

## 🔄 C-to-V Translation

Documentation for translating Zircon kernel from C to V:

- **[C-to-V Translation Guide](./translations/c2v_translations.md)** - Comprehensive translation guide
- **[Zircon C2V Workflow](./translations/zircon_c2v.md)** - Detailed workflow and tooling
- **[C2V Tooling Summary](./translations/C2V_TOOLING_SUMMARY.md)** - Tooling setup and usage
- **[HAL Translation](./translations/HAL_TRANSLATION_SUMMARY.md)** - HAL subsystem translation
- **[IPC Translation](./translations/IPC_TRANSLATION_SUMMARY.md)** - IPC subsystem translation
- **[VM Integration Guide](./translations/VM_INTEGRATION_GUIDE.md)** - Virtual memory subsystem
- **[VM Translation Files](./translations/VM_TRANSLATION_FILES.md)** - VM translation file listing
- **[VM Translation Report](./translations/VM_TRANSLATION_REPORT.md)** - VM translation status

## 📊 Project Reports

Historical reports and completion summaries:

- **[Ticket Completion Report](./reports/TICKET_COMPLETION_REPORT.md)** - c2v tooling implementation
- **[VM Ticket Summary](./reports/TICKET_VM_SUMMARY.md)** - VM subsystem ticket details

## 🎨 UI and Graphics

User interface and graphics subsystem documentation:

- **[Flatland Bindings](./ui/flatland_bindings.md)** - FIDL bindings for Flatland compositor
- **[Flatland Integration](./ui/flatland_integration.md)** - Integrating with Flatland

## 🎓 Tutorials

Step-by-step tutorials for getting started:

- **[Getting Started](./tutorials/getting_started.md)** - Complete setup and first build tutorial

## 🛠️ Tools & Scripts

Documentation for development tools and utilities:

- **[Tools Reference](./guides/tools_reference.md)** - Complete reference for all Soliloquy tools
- **[Build Manager](../tools/build_manager/README.md)** - Advanced build management system (GUI + CLI)
- **[Verification Scripts](../tools/scripts/README.md)** - Setup and subsystem verification tools

## 📦 Build System

- **[Build System Guide](./build.md)** - Comprehensive build documentation (GN, Bazel, Cargo)
- **[Build Manager](../tools/build_manager/README.md)** - Build management and automation
- **[C VM README](../third_party/zircon_c/vm/README.md)** - Original C sources

### Inter-Process Communication (IPC)
- **Location**: `third_party/zircon_v/ipc/`
- **[IPC README](../third_party/zircon_v/ipc/README.md)** - V translation of IPC subsystem
- **[C IPC README](../third_party/zircon_c/ipc/README.md)** - Original C sources

## Project Reports

- **[Ticket Completion Report](./TICKET_COMPLETION_REPORT.md)** - Completed work tracking

## Tools and Scripts

### Build Tools
- `tools/soliloquy/setup.sh` - Full Fuchsia source bootstrap
- `tools/soliloquy/setup_sdk.sh` - SDK-only setup
- `tools/soliloquy/env.sh` - Environment setup helper

### C-to-V Translation Tools
- `tools/soliloquy/c2v_pipeline.sh` - C-to-V translation pipeline
- `build/v_compile.py` - V compilation wrapper
- `build/v_translate.py` - C-to-V translation wrapper

### FIDL Tools
- `tools/soliloquy/gen_fidl_bindings.sh` - Generate Rust FIDL bindings

### Verification Scripts
- `verify_hal_v_translation.sh` - Verify HAL V translation
- `verify_vm_translation.sh` - Verify VM V translation  
- `verify_c2v_setup.sh` - Verify c2v tooling setup
- `verify_test_framework.sh` - Verify test framework

## Quick Command Reference

### Build Commands
```bash
# Bazel build
bazel build //target/path:target_name

# Build HAL
bazel build //drivers/common/soliloquy_hal:soliloquy_hal

# Build V translations
bazel build //third_party/zircon_v:zircon_v_hal
```

### Test Commands
```bash
# Run all tests
bazel test //...

# Run specific test suite
bazel test //drivers/common/soliloquy_hal/tests:all
bazel test //src/shell:soliloquy_shell_tests
```

### C-to-V Translation
```bash
# Bootstrap V toolchain
./tools/soliloquy/c2v_pipeline.sh --bootstrap-only

# Translate subsystem
./tools/soliloquy/c2v_pipeline.sh \
  --subsystem <name> \
  --sources third_party/zircon_c/<name> \
  --out-dir third_party/zircon_v/<name>

# Verify translation
./verify_hal_v_translation.sh
```

### Development Workflow
```bash
# Setup environment
./tools/soliloquy/setup_sdk.sh
source tools/soliloquy/env.sh

# Build and test
bazel build //...
bazel test //...

# Generate FIDL bindings
./tools/soliloquy/gen_fidl_bindings.sh
```

## Documentation Structure

```
docs/
├── INDEX.md                          # This file
├── c2v_translations.md               # C-to-V translation guide
├── zircon_c2v.md                     # Zircon c2v workflow
├── C2V_TOOLING_SUMMARY.md            # C2V tooling summary
├── HAL_TRANSLATION_SUMMARY.md        # HAL translation details
├── TICKET_COMPLETION_REPORT.md       # Project tracking
├── ARCHITECTURE.md                   # System architecture
├── dev_guide.md                      # Developer guide
├── build.md                          # Build documentation
├── TESTING.md                        # Testing overview
├── testing.md                        # Testing details
├── test_coverage_broadening.md       # Test coverage
├── getting_started_with_testing.md   # Testing quick start
├── QUICK_REFERENCE_MANIFEST.md       # Command reference
├── component_manifest.md             # Component manifests
├── driver_porting.md                 # Driver porting
├── servo_integration.md              # Servo integration
├── contributing.md                    # Contributing guide
└── ui/                               # UI documentation
    └── flatland_bindings.md          # FIDL bindings
```

## External Resources

- [Fuchsia Documentation](https://fuchsia.dev/)
- [Zircon Kernel Concepts](https://fuchsia.dev/fuchsia-src/concepts/kernel)
- [V Language Documentation](https://github.com/vlang/v/blob/master/doc/docs.md)
- [Bazel Documentation](https://bazel.build/)
- [GN Documentation](https://gn.googlesource.com/gn/+/main/docs/)

## Getting Help

If you can't find what you're looking for:

1. Check this index for related topics
2. Search the documentation directory: `grep -r "keyword" docs/`
3. Review the README files in subsystem directories
4. Check the verification scripts for examples

## Contributing to Documentation

When adding new documentation:

1. Add it to the `docs/` directory (not project root)
2. Update this INDEX.md with a link
3. Follow the existing documentation style
4. Include code examples where helpful
5. Add cross-references to related docs

