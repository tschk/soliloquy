# Soliloquy Development Guides

Comprehensive guides for developing Soliloquy components and contributing to the project.

## Available Guides

### [Developer Guide](./dev_guide.md) ⭐
**Complete development workflow and best practices**

Essential reading for all contributors. Covers:
- Development environment setup
- Code organization
- Contribution workflow
- Coding standards
- Review process

**Start here if you're new to Soliloquy development.**

---

### [Getting Started with Testing](./getting_started_with_testing.md)
**Comprehensive guide to testing in Soliloquy**

Learn how to write and run tests:
- Test frameworks (Rust, V, integration tests)
- Unit testing best practices
- Integration testing
- Running tests locally and in CI
- Test coverage tools

---

### [Driver Porting Guide](./driver_porting.md)
**Porting hardware drivers to Soliloquy**

Step-by-step guide for driver development:
- Driver architecture in Fuchsia/Soliloquy
- Porting existing drivers
- FIDL interface design
- DFv2 (Driver Framework v2)
- Testing and debugging drivers

**Example**: WiFi driver (AIC8800D80) porting walkthrough

---

### [Servo Integration Guide](./servo_integration.md)
**Integrating Servo browser engine**

How Servo powers Soliloquy's UI:
- Servo architecture overview
- WebRender integration
- WGPU graphics pipeline
- Input handling
- Component integration

---

### [Tools Reference](./tools_reference.md) 🛠️
**Complete reference for all development tools**

Comprehensive documentation for:
- Build tools (`build.sh`, `build_bazel.sh`, etc.)
- Development tools (`setup_sdk.sh`, `gen_fidl_bindings.sh`)
- Verification scripts
- Runtime and bridge checks
- Troubleshooting

**Use this as your command reference.**

---

## Quick Navigation

### By Experience Level

**Beginners** 🌱
1. [Getting Started Tutorial](../tutorials/getting_started.md)
2. [Developer Guide](./dev_guide.md)
3. [Tools Reference](./tools_reference.md)

**Intermediate** 🚀
1. [Getting Started with Testing](./getting_started_with_testing.md)
2. [Servo Integration Guide](./servo_integration.md)
3. [Architecture Documentation](../architecture/README.md)

**Advanced** 🔧
1. [Driver Porting Guide](./driver_porting.md)
2. [RV8 Linkage Roadmap](../rv8_linkage_roadmap.md)
3. [Build System Deep Dive](../build.md)

### By Task

**Setting up development environment**
→ [Tools Reference - Environment Setup](./tools_reference.md#environment-setup)

**Writing tests**
→ [Getting Started with Testing](./getting_started_with_testing.md)

**Porting a driver**
→ [Driver Porting Guide](./driver_porting.md)

**Building components**
→ [Tools Reference - Build Tools](./tools_reference.md#build-tools)

**Contributing code**
→ [Developer Guide](./dev_guide.md) + [Contributing](../contributing.md)

**Understanding architecture**
→ [Architecture Documentation](../architecture/README.md)

**Working on Servo/RV8 linkage**
→ [RV8 Linkage Roadmap](../rv8_linkage_roadmap.md)

---

## Guide Writing Guidelines

When contributing new guides:

### Structure
- Start with a clear overview
- Include table of contents for long guides
- Use practical examples
- Provide troubleshooting sections
- Link to related documentation

### Style
- Use clear, concise language
- Include code examples
- Use diagrams where helpful
- Highlight important information
- Keep commands copy-pasteable

### Format
```markdown
# Guide Title

Brief description (1-2 sentences).

## Table of Contents
- [Section 1](#section-1)
- [Section 2](#section-2)

## Prerequisites
- List requirements

## Section 1
Content...

## Troubleshooting
Common issues and solutions...

## See Also
- Related documentation links
```

---

## Contributing to Guides

Improvements are welcome! To contribute:

1. Fork the repository
2. Edit or create guide in `docs/guides/`
3. Follow the style guidelines above
4. Update this README if adding a new guide
5. Submit a pull request

See [Contributing Guide](../contributing.md) for details.

---

## See Also

- **[Tutorials](../tutorials/)** - Step-by-step tutorials
- **[Architecture](../architecture/)** - System architecture docs
- **[Testing](../testing/)** - Testing documentation
- **[Build Guide](../build.md)** - Build system reference
