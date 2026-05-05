# Contributing to Soliloquy

Thank you for your interest in contributing to Soliloquy! This document provides guidelines for contributing to the project.

## Quick Start

1. **Fork and clone** the repository
2. **Install dependencies** with Wax or the native Linux package manager
3. **Build UI**: `./tools/soliloquy/build_ui.sh`
4. **Test bridge**: `./tools/rv8_servo_test.sh bridge`
5. **Create a branch**: `git checkout -b feature/your-feature`
6. **Make changes** and commit
7. **Submit a PR** to `main` branch

## Development Environment

### Prerequisites
- macOS or Linux (Fedora recommended)
- Bazel 8.4.2+ (via Bazelisk)
- Rust 1.70+
- Bun
- 10GB+ free disk space

### Setup
```bash
# Clone repository
git clone https://github.com/atechnology-company/soliloquy.git
cd soliloquy

# Build desktop bundle
./tools/soliloquy/build_ui.sh

# Run targeted bridge checks
./tools/rv8_servo_test.sh bridge
```

## Code Style

### Rust
- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `rustfmt`: `cargo fmt`
- Use `clippy`: `cargo clippy`
- Maximum line length: 100 characters

### C++
- Follow [Google C++ Style Guide](https://google.github.io/styleguide/cppguide.html)
- Use `clang-format` with project config
- Maximum line length: 100 characters

### GN/Bazel
- Use tab indentation
- Keep BUILD files organized by type (libraries, binaries, tests)

## Commit Messages

Format:
```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `test`: Test additions/changes
- `refactor`: Code refactoring
- `chore`: Build/tooling changes

**Example:**
```
feat(wifi): add AIC8800 firmware loading

Implement firmware loading for AIC8800D80 WiFi chip using
Soliloquy HAL firmware utilities.

Closes #42
```

## Pull Request Process

1. **Create feature branch** from `main`
2. **Make changes** with clear commits
3. **Add tests** for new functionality
4. **Update documentation** as needed
5. **Run tests**: `cargo test` and `./tools/rv8_servo_test.sh bridge`
6. **Submit PR** with description of changes
7. **Address review feedback**
8. **Squash commits** if requested
9. **Merge** after approval

### PR Checklist
- [ ] Code follows style guidelines
- [ ] Tests added/updated
- [ ] Documentation updated
- [ ] All tests pass
- [ ] No merge conflicts
- [ ] Commit messages are clear

## Testing

### Unit Tests
```bash
# Run Rust tests
cargo test

# Run targeted bridge checks
./tools/rv8_servo_test.sh bridge

# Build desktop bundle
./tools/soliloquy/build_ui.sh
```

### Integration Tests
```bash
# Start local runtime
./tools/soliloquy/start.sh

# Test on hardware (if available)
./tools/soliloquy/debug.sh
```

## Project Structure

```
soliloquy/
├── src/              # Source code
│   └── shell/        # Soliloquy Shell
├── drivers/          # Device drivers
│   ├── wifi/         # WiFi drivers
│   └── gpio/         # GPIO driver
├── boards/           # Board configurations
│   └── arm64/        # ARM64 boards
├── third_party/      # Third-party code
│   └── servo/        # Servo browser engine
├── tools/            # Build and development tools
└── docs/             # Documentation
```

## Adding a New Component

1. Create directory: `src/your_component/`
2. Add source files
3. Create `BUILD.bazel`:
```python
rust_binary(
    name = "your_component",
    srcs = ["main.rs"],
    edition = "2021",
)
```
4. Add component manifest: `meta/your_component.cml`
5. Add tests
6. Update documentation

## Adding a New Driver

1. Create directory: `drivers/category/driver_name/`
2. Implement driver class with DDK integration
3. Create `BUILD.gn`:
```gn
driver("driver_name") {
  sources = [ "driver.cc" ]
  deps = [
    "//drivers/common/soliloquy_hal",
  ]
}
```
4. Add firmware files (if needed): `firmware/`
5. Create tests
6. Update board configuration

## Code Review

All submissions require review. We use GitHub pull requests for this purpose.

**Reviewers look for:**
- Code quality and style
- Test coverage
- Documentation
- Performance implications
- Security considerations
- Be nice

## Community

- **GitHub Issues**: Bug reports and feature requests
- **Discussions**: Questions and general discussion
- **Discord/Slack**: Real-time chat (link TBD)

## Questions?

- Open an issue for bugs or feature requests
- Start a discussion for questions
- Check existing documentation in `docs/`

Thank you for contributing to Soliloquy!
