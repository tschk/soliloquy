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

## Runtime and Appliance

Current runtime and operating-system appliance documentation:

- **[V0 Architecture](./v0-architecture.md)** - Alpine/OpenRC boot path, Servo surface, and `sold` system bridge
- **[Appliance System Architecture](./architecture/appliance-system.md)** - Immutable image, service policy, plugins, updates, and package management
- **[RV8 Linkage Roadmap](./rv8_linkage_roadmap.md)** - Servo/RV8 bridge status and remaining work
- **[API Contract](./api_contract.md)** - Local `sold` API contract

## 📊 Project Reports

Historical reports and completion summaries:

- **[Ticket Completion Report](./reports/TICKET_COMPLETION_REPORT.md)** - Historical completion tracking
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
- **[RV8/Servo Check](../tools/rv8_servo_test.sh)** - Targeted bridge validation helper

## 📦 Build System

- **[Build System Guide](./build.md)** - Comprehensive build documentation
- **[V0 Architecture](./v0-architecture.md)** - Alpine appliance boot and session architecture
- **[RV8 Linkage Roadmap](./rv8_linkage_roadmap.md)** - Servo/RV8 bridge work

## Project Reports

- **[Ticket Completion Report](./TICKET_COMPLETION_REPORT.md)** - Completed work tracking

## Tools and Scripts

### Runtime Tools
- `tools/soliloquy/start.sh` - Start the local Soliloquy UI and bridge services
- `tools/soliloquy/dev_ui.sh` - Start the desktop UI development server
- `tools/soliloquy/build_ui.sh` - Build the static desktop bundle for Servo

### Bridge Tools
- `tools/rv8_servo_test.sh` - Run targeted Servo/RV8 bridge checks

## Quick Command Reference

### Build Commands
```bash
# Bazel build
bazel build //target/path:target_name

# Build HAL
bazel build //drivers/common/soliloquy_hal:soliloquy_hal

# Build UI bundle
./tools/soliloquy/build_ui.sh
```

### Test Commands
```bash
# Run all tests
bazel test //...

# Run specific test suite
bazel test //drivers/common/soliloquy_hal/tests:all
bazel test //src/shell:soliloquy_shell_tests
```

### Runtime Development
```bash
# Start local UI and bridge services
./tools/soliloquy/start.sh

# Build the static desktop bundle
./tools/soliloquy/build_ui.sh

# Run targeted Servo/RV8 bridge checks
./tools/rv8_servo_test.sh bridge
```

### Development Workflow
```bash
# Start local runtime
./tools/soliloquy/start.sh

# Build desktop bundle
./tools/soliloquy/build_ui.sh

# Run bridge checks
./tools/rv8_servo_test.sh bridge
```

## Documentation Structure

```
docs/
├── INDEX.md                          # This file
├── ARCHITECTURE.md                   # System architecture
├── v0-architecture.md                # Alpine appliance architecture
├── rv8_linkage_roadmap.md            # Servo/RV8 bridge roadmap
├── api_contract.md                   # Local bridge API contract
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
