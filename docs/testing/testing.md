# Testing Guide

## Overview

Soliloquy uses Bazel for testing. Tests are co-located with source code.

## Running Tests

### All Tests
```bash
bazel test //...
```

### Specific Test
```bash
bazel test //vendor/servo:servo_mock_test
```

### With Coverage
```bash
bazel coverage //...
```

## Writing Tests

### Rust Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_feature() {
        let result = my_function();
        assert_eq!(result, expected);
    }
}
```

### Rust Integration Tests
Create `tests/` directory:
```rust
// tests/integration_test.rs
use my_crate::*;

#[test]
fn test_integration() {
    // Test code
}
```

### C++ Tests
```cpp
#include <gtest/gtest.h>

TEST(MyTest, TestCase) {
    EXPECT_EQ(1, 1);
}
```

## Test Organization

```
src/shell/
├── main.rs
├── servo_embedder.rs
└── BUILD.bazel        # Contains test targets
```

## BUILD File Example

```python
rust_library(
    name = "my_lib",
    srcs = ["lib.rs"],
)

rust_test(
    name = "my_lib_test",
    crate = ":my_lib",
)
```

## Best Practices

1. **Test Coverage** - Aim for >80% coverage
2. **Fast Tests** - Keep unit tests under 1 second
3. **Isolated** - Tests should not depend on each other
4. **Descriptive Names** - Use clear test names
5. **Assertions** - Use appropriate assertion macros

## Continuous Integration

Tests run automatically on:
- Pull requests
- Commits to main
- Nightly builds

## Hardware Testing

For driver testing on actual hardware:

```bash
# Flash device
./tools/soliloquy/debug.sh

# Connect serial console
./tools/soliloquy/debug.sh

# Run hardware tests
fx test soliloquy_wifi_driver_test
```
