#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "=== Running AIC8800 Mock Utilities Tests ==="
cd mock
cargo test --target x86_64-unknown-linux-gnu
cd ..

echo ""
echo "=== Running AIC8800 Integration Tests ==="
cd tests
cargo test --target x86_64-unknown-linux-gnu
cd ..

echo ""
echo "=== All tests passed! ==="
