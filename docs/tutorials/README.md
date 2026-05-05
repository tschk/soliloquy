# Soliloquy Tutorials

Step-by-step tutorials for learning Soliloquy development.

## Available Tutorials

### [Getting Started](./getting_started.md) ⭐
**Your first steps with Soliloquy development**

Perfect for newcomers! This tutorial covers:
- Setting up development environment
- Installing prerequisites
- Building the desktop bundle
- Running local bridge checks
- Building your first component
- Running tests
- Next steps for learning

**Time**: 30-60 minutes  
**Difficulty**: Beginner  
**Prerequisites**: Basic command line knowledge

---

## Coming Soon

### Component Development Tutorial (Planned)
Building a custom Soliloquy component from scratch.

**Topics**:
- Creating component structure
- Writing component manifest
- Implementing component logic
- FIDL interface design
- Testing components
- Packaging and deployment

---

### Driver Development Tutorial (Planned)
Writing and porting hardware drivers.

**Topics**:
- Driver framework overview
- DFv2 architecture
- Implementing driver interfaces
- Hardware interaction (MMIO, interrupts)
- Testing drivers
- Debugging techniques

---

### UI Development Tutorial (Planned)
Creating user interfaces with Servo and web technologies.

**Topics**:
- UI component structure
- HTML/CSS for Soliloquy UI
- JavaScript integration
- Flatland compositor integration
- Input handling
- Building interactive UIs

---

### Runtime Bridge Tutorial (Planned)
Extending the local bridge and Servo/RV8 runtime path.

**Topics**:
- Understanding `sold` API contracts
- Servo/RV8 bridge patterns
- Running targeted bridge checks
- Testing runtime integration
- Integration with build system
- Performance optimization

---

## Tutorial Guidelines

### For Learners

**How to use these tutorials**:
1. Start with [Getting Started](./getting_started.md)
2. Follow tutorials in order
3. Complete all exercises
4. Refer to guides when you need more detail
5. Join community if you get stuck

**Tips**:
- Don't skip prerequisites
- Type commands yourself (don't just copy-paste)
- Experiment beyond tutorial examples
- Ask questions if confused
- Take breaks on long tutorials

### For Tutorial Writers

**Creating effective tutorials**:

#### Structure
```markdown
# Tutorial Title

Brief description (1-2 sentences).

**Time**: X minutes/hours
**Difficulty**: Beginner/Intermediate/Advanced
**Prerequisites**: List requirements

## What You'll Learn
- Learning objective 1
- Learning objective 2
- Learning objective 3

## Step 1: Title
Explanation...
```bash
command
```
Expected output...

## Step 2: Title
...

## Troubleshooting
Common issues...

## Next Steps
- Link to next tutorial
- Related documentation

## See Also
- Related links
```

#### Best Practices
- **Clear objectives**: State what learner will accomplish
- **Prerequisites**: List required knowledge/setup
- **Step-by-step**: Break into digestible steps
- **Show output**: Include expected command output
- **Explain why**: Don't just show how, explain reasoning
- **Troubleshoot**: Include common problems and solutions
- **Test thoroughly**: Verify tutorial works on fresh system
- **Keep updated**: Update when tools/APIs change

#### Tutorial Checklist
- [ ] Clear learning objectives stated
- [ ] Prerequisites listed
- [ ] Time estimate provided
- [ ] Difficulty level specified
- [ ] Steps are numbered and clear
- [ ] Commands are copy-pasteable
- [ ] Expected outputs shown
- [ ] Concepts explained (not just commands)
- [ ] Troubleshooting section included
- [ ] Next steps provided
- [ ] Tested on fresh environment
- [ ] Links to related docs included

---

## Tutorial Topics We Need

Vote on or contribute tutorials for these topics:

### Development
- [ ] Creating your first component
- [ ] Writing FIDL interfaces
- [ ] Component testing strategies
- [ ] Debugging components
- [ ] Using runtime helper scripts

### Drivers
- [ ] Porting a simple driver
- [ ] Driver testing and debugging
- [ ] DMA and interrupt handling
- [ ] Platform-specific driver development

### Runtime
- [ ] Local bridge basics
- [ ] Extending a runtime endpoint
- [ ] Testing bridge behavior
- [ ] Performance tuning bridge code

### UI/Graphics
- [ ] Building a simple UI component
- [ ] Integrating with Flatland
- [ ] WebGPU graphics programming
- [ ] Input event handling

### Advanced
- [ ] Custom build configurations
- [ ] Cross-compilation for ARM64
- [ ] Remote debugging
- [ ] Performance profiling
- [ ] Security hardening

---

## Learning Paths

### Web Developer → Soliloquy Developer
1. [Getting Started](./getting_started.md)
2. UI Development Tutorial (coming soon)
3. [Servo Integration Guide](../guides/servo_integration.md)
4. Component Development Tutorial (coming soon)

### Systems Programmer → Soliloquy Developer
1. [Getting Started](./getting_started.md)
2. [Architecture Overview](../architecture/README.md)
3. [Driver Porting Guide](../guides/driver_porting.md)
4. Runtime Bridge Tutorial (coming soon)

### General Developer → Soliloquy Developer
1. [Getting Started](./getting_started.md)
2. [Developer Guide](../guides/dev_guide.md)
3. Component Development Tutorial (coming soon)
4. [Testing Guide](../guides/getting_started_with_testing.md)

---

## Additional Resources

### Documentation
- [Developer Guide](../guides/dev_guide.md) - Comprehensive development guide
- [Architecture](../architecture/README.md) - System architecture
- [Tools Reference](../guides/tools_reference.md) - Tool documentation
- [API Documentation](#) - Generated API docs (coming soon)

### Community
- GitHub Issues - Bug reports and feature requests
- Discussions - Questions and community help
- Contributing Guide - How to contribute

### External Resources
- [Fuchsia Documentation](https://fuchsia.dev/)
- [V Language Documentation](https://vlang.io/)
- [Servo Documentation](https://servo.org/)
- [Bazel Documentation](https://bazel.build/)

---

## Contributing Tutorials

We welcome tutorial contributions! To add a tutorial:

1. **Plan your tutorial**
   - Define learning objectives
   - Identify target audience
   - Outline steps

2. **Write tutorial**
   - Follow structure guidelines above
   - Include all code examples
   - Test on fresh environment

3. **Submit for review**
   - Create PR with tutorial file
   - Update this README
   - Link related documentation

4. **Address feedback**
   - Incorporate review comments
   - Test suggested improvements
   - Update as needed

See [Contributing Guide](../contributing.md) for process details.

---

## Getting Help

If you get stuck on a tutorial:

1. Check **Troubleshooting** section in tutorial
2. Review [Tools Reference](../guides/tools_reference.md) for command details
3. Search existing GitHub Issues
4. Ask in GitHub Discussions
5. File an issue with "tutorial" label

When asking for help, include:
- Which tutorial and step
- Commands you ran
- Error messages (full output)
- Your OS and setup details

---

**Start learning**: [Getting Started Tutorial](./getting_started.md) →
