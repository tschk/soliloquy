# Soliloquy Testing Documentation

Comprehensive testing documentation for Soliloquy OS.

## Documents

### [Testing Guide](./testing.md)
Complete testing documentation covering all aspects of testing in Soliloquy.

**Topics**:
- Testing philosophy and strategy
- Test frameworks (Rust, V, Bazel)
- Unit testing
- Integration testing
- System testing
- Test organization
- CI/CD integration

### [Test Coverage Expansion](./test_coverage_broadening.md)
Strategies and guidelines for expanding test coverage across the project.

**Topics**:
- Coverage analysis
- Identifying gaps
- Writing effective tests
- Coverage tools
- Coverage targets
- Best practices

## Testing Strategy

### Test Pyramid

```
        ┌────────────────┐
        │  System Tests  │  ← Few, slow, expensive
        └────────────────┘
      ┌──────────────────────┐
      │  Integration Tests   │  ← Some, medium speed
      └──────────────────────┘
  ┌──────────────────────────────┐
  │       Unit Tests             │  ← Many, fast, cheap
  └──────────────────────────────┘
```

### Test Levels

1. **Unit Tests** (70%)
   - Test individual functions/modules
   - Fast execution (<1s per test)
   - No external dependencies
   - Mock/stub interfaces

2. **Integration Tests** (20%)
   - Test component interactions
   - FIDL interface testing
   - Multi-component scenarios
   - Medium execution time

3. **System Tests** (10%)
   - End-to-end testing
   - Full system integration
   - Hardware testing (where applicable)
   - Slow execution

## Running Tests

### Quick Start

```bash
# Run all tests
bazel test //...

# Run specific subsystem tests
bazel test //test/vm:...
bazel test //test/hal:...

# Run with coverage
bazel coverage //...

# Rust tests
cargo test --workspace
```

### Advanced Options

```bash
# Verbose output
bazel test //... --test_output=all

# Run specific test
bazel test //test/vm:vm_object_test

# Run tests matching pattern
bazel test //... --test_filter="*vm*"

# Run tests in parallel
bazel test //... --jobs=8

# Debug failed tests
bazel test //... --test_output=errors --verbose_failures
```

## Test Organization

```
test/
├── unit/              # Unit tests
│   ├── vm/           # VM subsystem tests
│   ├── hal/          # HAL tests
│   └── ipc/          # IPC tests
├── integration/       # Integration tests
│   ├── component/    # Component interaction tests
│   └── fidl/         # FIDL interface tests
└── system/           # System-level tests
    ├── boot/         # Boot sequence tests
    └── scenarios/    # End-to-end scenarios
```

## Writing Tests

### Rust Unit Test Example

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_object_creation() {
        let vm_obj = VmObject::new(4096);
        assert_eq!(vm_obj.size(), 4096);
    }

    #[test]
    #[should_panic(expected = "invalid size")]
    fn test_vm_object_invalid_size() {
        VmObject::new(0);
    }
}
```

### Bazel Test Target

```python
# BUILD.bazel
rust_test(
    name = "vm_object_test",
    srcs = ["vm_object_test.rs"],
    deps = [
        "//src/vm:vm_object",
        "@crates//:anyhow",
    ],
)
```

### V Language Test Example

```v
module vm_test

import vm

fn test_page_allocation() {
    page := vm.allocate_page()
    assert page.size == 4096
    assert page.is_aligned()
}

fn test_page_deallocation() {
    page := vm.allocate_page()
    vm.free_page(page)
    // Verify page is freed
}
```

## Test Coverage

### Current Coverage Status

| Subsystem | Unit Tests | Integration Tests | Coverage |
|-----------|------------|-------------------|----------|
| VM        | ✅ High    | ✅ Medium         | 75%      |
| HAL       | ⚠️ Medium  | ⚠️ Low            | 45%      |
| IPC       | ✅ High    | ✅ High           | 80%      |
| Shell     | ⚠️ Medium  | ⚠️ Medium         | 60%      |
| Drivers   | ❌ Low     | ❌ Low            | 25%      |

**Target**: 80% overall coverage

### Generating Coverage Reports

```bash
# Generate coverage report
bazel coverage //...

# View HTML report
genhtml bazel-out/_coverage/_coverage_report.dat -o coverage_html
open coverage_html/index.html

# Or use coverage tool
./tools/rv8_servo_test.sh bridge
```

## Continuous Integration

Tests run automatically on:
- Every pull request
- Commits to main branch
- Nightly builds

### CI Test Pipeline

```
1. Environment Setup
   ↓
2. Unit Tests (parallel)
   ↓
3. Integration Tests
   ↓
4. Coverage Analysis
   ↓
5. Report Generation
```

### CI Configuration

See `.github/workflows/test.yml` for CI setup.

## Test Utilities

### Mocking Frameworks

**Rust**: `mockall` crate
```rust
use mockall::*;

#[automock]
trait VmAllocator {
    fn allocate(&self, size: usize) -> Result<Page>;
}

#[test]
fn test_with_mock() {
    let mut mock = MockVmAllocator::new();
    mock.expect_allocate()
        .returning(|_| Ok(Page::new(4096)));
}
```

### Test Fixtures

**Rust**: Test fixtures for common setup
```rust
struct TestFixture {
    vm: VmSubsystem,
    pages: Vec<Page>,
}

impl TestFixture {
    fn new() -> Self {
        Self {
            vm: VmSubsystem::new(),
            pages: vec![],
        }
    }
}

#[test]
fn test_with_fixture() {
    let fixture = TestFixture::new();
    // Use fixture
}
```

## Test Best Practices

### DO ✅
- Write tests before/with code (TDD)
- Keep tests simple and focused
- Use descriptive test names
- Test edge cases and error conditions
- Mock external dependencies
- Keep tests fast
- Make tests deterministic

### DON'T ❌
- Write flaky tests
- Test implementation details
- Share state between tests
- Ignore failing tests
- Write slow tests in unit test suite
- Use hardcoded paths or values
- Commit commented-out tests

## Debugging Tests

### Debug Individual Test

```bash
# Run with debugger
bazel run --run_under=gdb //test/vm:vm_object_test

# Add debug output
RUST_LOG=debug bazel test //test/vm:vm_object_test

# Run without optimization for debugging
bazel test //... --compilation_mode=dbg
```

### Common Issues

**Test fails intermittently**
- Check for race conditions
- Verify no shared state
- Look for timing dependencies

**Test is slow**
- Profile test execution
- Check for unnecessary I/O
- Consider moving to integration tests

**Test fails in CI but passes locally**
- Check for environment dependencies
- Verify deterministic behavior
- Review CI logs carefully

## Performance Testing

### Benchmarking

```rust
#[bench]
fn bench_vm_allocation(b: &mut Bencher) {
    b.iter(|| {
        VmObject::new(4096)
    });
}
```

Run benchmarks:
```bash
cargo bench
```

## Test Documentation

Document tests with:
- Purpose of the test
- Setup requirements
- Expected behavior
- Edge cases covered

```rust
/// Tests that VM object creation properly initializes size
/// and alignment for page-sized allocations.
///
/// Edge cases:
/// - Minimum size (1 page)
/// - Power-of-2 sizes
/// - Alignment requirements
#[test]
fn test_vm_object_page_aligned_creation() {
    // Test implementation
}
```

## See Also

- **[Getting Started with Testing](../guides/getting_started_with_testing.md)** - Testing tutorial
- **[Developer Guide](../guides/dev_guide.md)** - Development workflow
- **[Tools Reference](../guides/tools_reference.md)** - Test tools documentation
- **[Contributing Guide](../contributing.md)** - Contribution guidelines
