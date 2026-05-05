# Soliloquy OS Documentation

Welcome to Soliloquy OS documentation! This directory contains comprehensive documentation for developers, contributors, and users.

## 📖 Quick Start

**New to Soliloquy?** Start here:
1. **[Getting Started Tutorial](./tutorials/getting_started.md)** - Complete setup and first build
2. **[Developer Guide](./guides/dev_guide.md)** - Development workflow
3. **[Tools Reference](./guides/tools_reference.md)** - Tool documentation

## 📂 Documentation Structure

### [Tutorials](./tutorials/) 🎓
Step-by-step guides for learning Soliloquy development.
- **[Getting Started](./tutorials/getting_started.md)** - Your first build (30-60 min)

### [Guides](./guides/) 📚
Comprehensive guides for specific development tasks.
- **[Developer Guide](./guides/dev_guide.md)** - Complete development workflow
- **[Tools Reference](./guides/tools_reference.md)** - All tools documented
- **[Testing Guide](./guides/getting_started_with_testing.md)** - Testing strategies
- **[Driver Porting](./guides/driver_porting.md)** - Porting hardware drivers
- **[Servo Integration](./guides/servo_integration.md)** - Browser engine integration

### [Architecture](./architecture/) 🏗️
System design and component architecture.
- **[System Architecture](./architecture/architecture.md)** - High-level design
- **[Component Manifests](./architecture/component_manifest.md)** - Component structure
- **[Quick Reference](./architecture/quick_reference_manifest.md)** - Command reference

### [Testing](./testing/) 🧪
Testing documentation and strategies.
- **[Testing Overview](./testing/testing.md)** - Testing strategy
- **[Test Coverage](./testing/test_coverage_broadening.md)** - Coverage expansion

### Runtime and Appliance
Current appliance and browser-runtime documentation.
- **[V0 Architecture](./v0-architecture.md)** - Alpine/OpenRC boot path, Servo surface, and `sold` bridge
- **[Appliance System Architecture](./architecture/appliance-system.md)** - Immutable image, service policy, plugins, updates, and package management
- **[RV8 Linkage Roadmap](./rv8_linkage_roadmap.md)** - Servo/RV8 bridge status and remaining work
- **[API Contract](./api_contract.md)** - Local `sold` API contract

### [UI Documentation](./ui/) 🎨
User interface and graphics documentation.
- **[Flatland Bindings](./ui/flatland_bindings.md)** - Compositor FIDL bindings
- **[Flatland Integration](./ui/flatland_integration.md)** - Compositor integration

### Root Documentation
- **[INDEX.md](./INDEX.md)** - Complete documentation index with all links
- **[build.md](./build.md)** - Comprehensive build system documentation
- **[contributing.md](./contributing.md)** - Contribution guidelines and workflow

## 🗺️ Documentation Map by Role

### I'm a New Developer
1. [Getting Started Tutorial](./tutorials/getting_started.md)
2. [Developer Guide](./guides/dev_guide.md)
3. [Tools Reference](./guides/tools_reference.md)
4. [Contributing Guide](./contributing.md)

### I'm Contributing Code
1. [Developer Guide](./guides/dev_guide.md)
2. [Testing Guide](./guides/getting_started_with_testing.md)
3. [Architecture Overview](./architecture/architecture.md)
4. [Contributing Guidelines](./contributing.md)

### I'm Working on Drivers
1. [Driver Porting Guide](./guides/driver_porting.md)
2. [Architecture Documentation](./architecture/README.md)
3. [Testing Guide](./testing/testing.md)
4. [Tools Reference](./guides/tools_reference.md)

### I'm Working on UI
1. [Servo Integration Guide](./guides/servo_integration.md)
2. [UI Documentation](./ui/)
3. [Architecture - Component Model](./architecture/architecture.md)
4. [Component Manifests](./architecture/component_manifest.md)

### I'm Working on Runtime Integration
1. [V0 Architecture](./v0-architecture.md)
2. [RV8 Linkage Roadmap](./rv8_linkage_roadmap.md)
3. [Servo Integration Guide](./guides/servo_integration.md)
4. [Tools Reference](./guides/tools_reference.md)

### I'm Setting Up Build System
1. [Build Guide](./build.md)
2. [Tools Reference](./guides/tools_reference.md)
3. [Getting Started Tutorial](./tutorials/getting_started.md)
4. [Developer Guide](./guides/dev_guide.md)

## 🔍 Finding Documentation

### By Topic

| Topic | Documents |
|-------|-----------|
| **Setup** | [Getting Started](./tutorials/getting_started.md), [Tools Reference](./guides/tools_reference.md) |
| **Building** | [Build Guide](./build.md), [Tools Reference](./guides/tools_reference.md) |
| **Testing** | [Testing Overview](./testing/testing.md), [Getting Started with Testing](./guides/getting_started_with_testing.md) |
| **Architecture** | [System Architecture](./architecture/architecture.md), [Components](./architecture/component_manifest.md) |
| **Drivers** | [Driver Porting](./guides/driver_porting.md), [Architecture Docs](./architecture/README.md) |
| **UI** | [Servo Integration](./guides/servo_integration.md), [UI Docs](./ui/) |
| **Runtime** | [V0 Architecture](./v0-architecture.md), [RV8 Linkage Roadmap](./rv8_linkage_roadmap.md), [API Contract](./api_contract.md) |
| **Tools** | [Tools Reference](./guides/tools_reference.md), [RV8/Servo Check](../tools/rv8_servo_test.sh) |

### By Question

| Question | Answer |
|----------|--------|
| How do I get started? | [Getting Started Tutorial](./tutorials/getting_started.md) |
| How do I build? | [Build Guide](./build.md) |
| What tools are available? | [Tools Reference](./guides/tools_reference.md) |
| How do I run tests? | [Testing Guide](./guides/getting_started_with_testing.md) |
| How does it work? | [Architecture](./architecture/architecture.md) |
| How do I contribute? | [Contributing Guide](./contributing.md) |
| How do I port a driver? | [Driver Porting Guide](./guides/driver_porting.md) |
| How does the local system bridge work? | [API Contract](./api_contract.md) |

## 📝 Documentation Standards

### Writing Style
- **Clear and concise** - Get to the point quickly
- **Actionable** - Include commands and examples
- **Well-structured** - Use headings and lists
- **Up-to-date** - Keep documentation current with code

### Format Guidelines
- Use Markdown (`.md`) files
- Include table of contents for long docs
- Provide code examples with syntax highlighting
- Link to related documentation
- Include troubleshooting sections

### Documentation Types

**Tutorials** - Learning-oriented
- Step-by-step instructions
- Complete examples
- Expected outputs shown
- Troubleshooting included

**Guides** - Task-oriented
- How to accomplish specific goals
- Best practices
- Multiple approaches when applicable
- Links to references

**Reference** - Information-oriented
- Complete and accurate information
- Technical details
- API documentation
- Command references

**Architecture** - Understanding-oriented
- System design explanations
- Diagrams and illustrations
- Design decisions and rationale
- Component relationships

## 🤝 Contributing to Documentation

Documentation contributions are highly valued! To contribute:

1. **Find what needs improvement**
   - Unclear instructions
   - Missing documentation
   - Outdated information
   - Broken links

2. **Make your changes**
   - Edit existing docs or create new ones
   - Follow structure and style guidelines
   - Add to appropriate directory
   - Update indexes and READMEs

3. **Submit pull request**
   - Clear description of changes
   - Why the change is needed
   - Link to related issues if applicable

See [Contributing Guide](./contributing.md) for complete process.

## 🔧 Documentation Tools

### Runtime Checks
Validate targeted Servo/RV8 bridge behavior.
- **Location**: `../tools/rv8_servo_test.sh`
- **Docs**: [Tools Reference](./guides/tools_reference.md)

### Development Tools
Build, test, and deploy tools.
- **Location**: `../tools/soliloquy/`
- **Docs**: [Tools Reference](./guides/tools_reference.md)

## 📚 External Resources

### Fuchsia Documentation
- [Fuchsia.dev](https://fuchsia.dev/) - Official Fuchsia documentation
- [Zircon Concepts](https://fuchsia.dev/fuchsia-src/concepts/kernel) - Kernel documentation

### Servo
- [Servo.org](https://servo.org/) - Servo project
- [Servo Book](https://book.servo.org/) - Servo documentation

### Build Systems
- [Bazel](https://bazel.build/) - Bazel build system
- [GN Reference](https://gn.googlesource.com/gn/+/main/docs/reference.md) - GN documentation

## 🆘 Getting Help

### Documentation Questions
- Check [INDEX.md](./INDEX.md) for complete documentation list
- Search GitHub issues for similar questions
- Ask in GitHub Discussions

### Development Help
- Review [Developer Guide](./guides/dev_guide.md)
- Check [Tools Reference](./guides/tools_reference.md)
- Consult [Architecture docs](./architecture/README.md)

### Reporting Issues
- Documentation bugs: File with "documentation" label
- Unclear instructions: File with "documentation" and "help wanted" labels
- Suggestions: Use GitHub Discussions

## 📈 Documentation Status

| Category | Status | Coverage |
|----------|--------|----------|
| Tutorials | 🟡 Basic | 1 tutorial |
| Guides | 🟢 Good | 5 guides |
| Architecture | 🟢 Good | Complete |
| Testing | 🟢 Good | Complete |
| Runtime | 🟢 Good | Active roadmap |
| API Reference | 🔴 Missing | Planned |
| Tools | 🟢 Excellent | Complete |

**Legend**: 🟢 Good | 🟡 Needs Work | 🔴 Missing

---

**Ready to start?** → [Getting Started Tutorial](./tutorials/getting_started.md)
