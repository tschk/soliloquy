# Soliloquy Tools Reference

Complete reference for all tools and scripts in the Soliloquy project.

## Table of Contents

- [Build Tools](#build-tools)
- [Development Tools](#development-tools)
- [Testing & Verification Tools](#testing--verification-tools)
- [Build Manager](#build-manager)
- [Environment Setup](#environment-setup)

---

## Build Tools

### `tools/soliloquy/build.sh`
Full Fuchsia build with Soliloquy components (Linux only).

```bash
./tools/soliloquy/build.sh
```

**Prerequisites**: Fuchsia source tree bootstrapped via `setup.sh`

### `tools/soliloquy/build_sdk.sh`
SDK-based cross-platform build for Soliloquy components.

```bash
./tools/soliloquy/build_sdk.sh
```

**Prerequisites**: SDK downloaded via `setup_sdk.sh`

### `tools/soliloquy/build_bazel.sh`
Bazel-based component build.

```bash
./tools/soliloquy/build_bazel.sh [target]

# Examples:
./tools/soliloquy/build_bazel.sh //src/shell:soliloquy_shell
./tools/soliloquy/build_bazel.sh //...  # Build all targets
```

### `tools/soliloquy/ssh_build.sh`
Remote build from macOS to Linux server.

```bash
./tools/soliloquy/ssh_build.sh user@linux-server
```

**Use case**: macOS users building on remote Linux machines

---

## Development Tools

### `tools/soliloquy/setup.sh`
Bootstrap full Fuchsia source tree (Linux only).

```bash
./tools/soliloquy/setup.sh
```

**What it does**:
- Installs system dependencies
- Clones Fuchsia repository
- Bootstraps build system
- Links Soliloquy sources

**Disk space**: ~50GB required

### `tools/soliloquy/setup_sdk.sh`
Download and configure Fuchsia SDK (cross-platform).

```bash
# Latest version
./tools/soliloquy/setup_sdk.sh

# Specific version
export FUCHSIA_SDK_VERSION=20231115.2
./tools/soliloquy/setup_sdk.sh
```

**Disk space**: ~2.3GB

### `tools/soliloquy/env.sh`
Set up environment variables for development.

```bash
source tools/soliloquy/env.sh
```

**Sets**:
- `FUCHSIA_SDK_PATH`
- `PATH` updates for SDK tools
- Build system configuration

### `tools/soliloquy/gen_fidl_bindings.sh`
Generate FIDL bindings for V language.

```bash
./tools/soliloquy/gen_fidl_bindings.sh [library]

# Examples:
./tools/soliloquy/gen_fidl_bindings.sh fuchsia.ui.composition
./tools/soliloquy/gen_fidl_bindings.sh  # Generate all
```

### `tools/soliloquy/c2v_pipeline.sh`
C-to-V translation pipeline for Zircon subsystems.

```bash
./tools/soliloquy/c2v_pipeline.sh --subsystem <name> [OPTIONS]

# Options:
#   --subsystem <name>   Target subsystem (hal, vm, ipc)
#   --dry-run           Show plan without executing
#   --out-dir <path>    Output directory (default: out/c2v)
#   --bootstrap-only    Only setup V toolchain
#   --help             Show help

# Examples:
./tools/soliloquy/c2v_pipeline.sh --subsystem hal
./tools/soliloquy/c2v_pipeline.sh --subsystem vm --dry-run
```

### `tools/soliloquy/validate_manifest.sh`
Validate component manifest files.

```bash
./tools/soliloquy/validate_manifest.sh [manifest_file]

# Examples:
./tools/soliloquy/validate_manifest.sh src/shell/meta/soliloquy_shell.cml
```

---

## Testing & Verification Tools

### `tools/soliloquy/test.sh`
Run project tests.

```bash
./tools/soliloquy/test.sh [target]

# Examples:
./tools/soliloquy/test.sh              # Run all tests
./tools/soliloquy/test.sh //test/vm:*  # Run VM tests
```

### Verification Scripts

All verification scripts are located in `tools/scripts/` and can be run from the project root:

#### `tools/scripts/verify_c2v_setup.sh`
Verify C-to-V translation tooling setup.

```bash
./tools/scripts/verify_c2v_setup.sh
```

**Checks**:
- c2v_pipeline.sh functionality
- GN build files
- Bazel build files
- Python wrapper scripts

#### `tools/scripts/verify_hal_v_translation.sh`
Verify HAL subsystem V translation.

```bash
./tools/scripts/verify_hal_v_translation.sh
```

**Checks**:
- HAL V translation files exist
- Build system integration
- Translation completeness

#### `tools/scripts/verify_ipc_build.sh`
Verify IPC subsystem build configuration.

```bash
./tools/scripts/verify_ipc_build.sh
```

**Checks**:
- IPC build targets
- Build file correctness
- Dependencies

#### `tools/scripts/verify_test_framework.sh`
Verify testing framework setup.

```bash
./tools/scripts/verify_test_framework.sh
```

**Checks**:
- Test infrastructure
- Test runners
- Test targets availability

#### `tools/scripts/verify_vm_translation.sh`
Verify VM subsystem V translation.

```bash
./tools/scripts/verify_vm_translation.sh
```

**Checks**:
- VM V translation files
- Build integration
- Translation status

---

## Build Manager

The Build Manager is a comprehensive build management system with both GUI and CLI interfaces.

### CLI Usage

```bash
# Build management
soliloquy-build start [target] --system [gn|bazel|cargo]
soliloquy-build stop [build-id]
soliloquy-build status
soliloquy-build clean

# Module operations
soliloquy-build module list
soliloquy-build module info <name>
soliloquy-build module deps <name>
soliloquy-build module build <name>

# Testing
soliloquy-build test run [pattern]
soliloquy-build test list
soliloquy-build test report

# Development tools
soliloquy-build fidl generate [library]
soliloquy-build c2v translate <subsystem>
soliloquy-build env setup
soliloquy-build env check

# Analytics
soliloquy-build stats
soliloquy-build history [days]
soliloquy-build compare <build-id-1> <build-id-2>

# Profiles
soliloquy-build profile save <name>
soliloquy-build profile load <name>
soliloquy-build profile list
```

### GUI Application

```bash
# Launch GUI
soliloquy-build-manager

# Or from source
cd tools/build_manager/build_manager_gui
bun run tauri dev
```

**See**: [Build Manager Documentation](../../tools/build_manager/README.md)

---

## Hardware Tools

### `tools/soliloquy/flash.sh`
Flash OS image to target device.

```bash
./tools/soliloquy/flash.sh [image]

# Examples:
./tools/soliloquy/flash.sh              # Flash default Soliloquy image
./tools/soliloquy/flash.sh custom.img   # Flash custom image
```

**Prerequisites**: Device connected via USB or network

### `tools/soliloquy/debug.sh`
Connect to device serial console for debugging.

```bash
./tools/soliloquy/debug.sh
```

**Prerequisites**: Serial connection configured

---

## UI Development Tools

### `tools/soliloquy/dev_ui.sh`
Start UI prototype development server.

```bash
./tools/soliloquy/dev_ui.sh
```

**What it does**:
- Runs the SvelteKit dev server via Bun (Servo desktop shell)
- Provides hot reload + HMR for the Servo/V8 runtime surface
- Binds to port 5173 by default (override with VITE_PORT)

### `tools/soliloquy/build_ui.sh`
Build UI prototype for production.

```bash
./tools/soliloquy/build_ui.sh
```

---

## Environment Setup

### First-Time Setup (SDK-based, Recommended)

```bash
# 1. Download SDK
./tools/soliloquy/setup_sdk.sh

# 2. Set up environment
source tools/soliloquy/env.sh

# 3. Verify setup
./tools/scripts/verify_test_framework.sh

# 4. Build components
./tools/soliloquy/build_bazel.sh //...
```

### Full Source Setup (Linux only, Advanced)

```bash
# 1. Bootstrap Fuchsia
./tools/soliloquy/setup.sh

# 2. Build
./tools/soliloquy/build.sh

# 3. Flash to device
./tools/soliloquy/flash.sh
```

---

## Tool Troubleshooting

### Common Issues

#### "Command not found"
Ensure you've sourced the environment:
```bash
source tools/soliloquy/env.sh
```

#### "SDK not found"
Download the SDK:
```bash
./tools/soliloquy/setup_sdk.sh
```

#### "Bazel not found"
Install Bazelisk:
```bash
# macOS
brew install bazelisk

# Linux
wget https://github.com/bazelbuild/bazelisk/releases/latest/download/bazelisk-linux-amd64
chmod +x bazelisk-linux-amd64
sudo mv bazelisk-linux-amd64 /usr/local/bin/bazel
```

#### "Permission denied"
Make scripts executable:
```bash
chmod +x tools/soliloquy/*.sh
chmod +x tools/scripts/*.sh
```

---

## Tool Development

### Adding New Tools

1. Create script in `tools/soliloquy/` or `tools/scripts/`
2. Make executable: `chmod +x script.sh`
3. Add to this documentation
4. Update build_manager integration if applicable

### Tool Script Guidelines

- Use `#!/bin/bash` shebang
- Include `set -e` for error handling
- Add help text with `--help` flag
- Detect project root properly
- Provide clear error messages
- Log operations for debugging

---

## See Also

- [Build Guide](../build.md) - Detailed build system documentation
- [Developer Guide](./dev_guide.md) - Complete development workflow
- [Build Manager README](../../tools/build_manager/README.md) - Build Manager details
