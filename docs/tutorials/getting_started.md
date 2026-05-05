# Getting Started with Soliloquy Development

This tutorial walks through the active local development path for the Alpine appliance, Servo desktop surface, `sold` bridge, and RV8/Servo linkage work.

## Prerequisites

Before you begin, ensure you have:

- **Operating System**: macOS 10.15+ or Linux
- **Package manager**: Wax on macOS
- **Toolchains**: Rust, Bun, and the native C/C++ toolchain needed by Servo checks
- **Disk Space**: Enough for the repository, UI dependencies, and local build artifacts

## Step 1: Clone the Repository

```bash
git clone https://github.com/yourusername/soliloquy.git
cd soliloquy
```

## Step 2: Install Host Dependencies

### macOS

```bash
wax --version
wax install bazelisk
```

### Linux

Use the native distro package manager for Rust, Bun, C/C++ build tools, Python, curl, wget, and git.

## Step 3: Build the Desktop Bundle

```bash
./tools/soliloquy/build_ui.sh
```

This builds the static desktop bundle that Servo loads as the appliance UI.

## Step 4: Run the UI Development Server

```bash
./tools/soliloquy/dev_ui.sh
```

This launches the Servo/V8 desktop shell UI with Bun-backed development serving.

## Step 5: Start the Local Runtime

```bash
./tools/soliloquy/start.sh
```

This starts the local runtime path used by the desktop appliance workflow.

## Step 6: Run Targeted Bridge Checks

```bash
./tools/rv8_servo_test.sh bridge
```

This validates the narrow Servo/RV8 bridge path and schema parser coverage.

## Next Steps

1. **Read the Developer Guide**: [guides/dev_guide.md](../guides/dev_guide.md)
2. **Review the Appliance Architecture**: [v0-architecture.md](../v0-architecture.md)
3. **Track RV8 linkage work**: [rv8_linkage_roadmap.md](../rv8_linkage_roadmap.md)
4. **Use the Tools Reference**: [guides/tools_reference.md](../guides/tools_reference.md)

## Troubleshooting

### UI build fails

Run the UI build directly through Bun:

```bash
cd ui/desktop
bun run build
```

### Bridge check fails on macOS

The Servo/RV8 check may require the local Xcode SDK and `libclang` environment shim described in [RV8 Linkage Roadmap](../rv8_linkage_roadmap.md).

### Target debugging

Use the serial debug helper when a device is connected:

```bash
./tools/soliloquy/debug.sh
```
