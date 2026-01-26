# AIC8800 Driver - Quick Start Guide

## Testing the Mock Utilities and Integration Tests

The AIC8800 driver includes comprehensive Rust-based mock utilities and integration tests that can be run without hardware.

### Run All Tests (Recommended)

```bash
cd drivers/wifi/aic8800
./run_tests.sh
```

Expected output:
```
=== Running AIC8800 Mock Utilities Tests ===
running 19 tests
... (all tests pass)
test result: ok. 19 passed

=== Running AIC8800 Integration Tests ===
running 10 tests
... (all tests pass)
test result: ok. 10 passed

=== All tests passed! ===
```

### Run Individual Test Suites

**Mock Utilities Tests:**
```bash
cd drivers/wifi/aic8800/mock
cargo test --target x86_64-unknown-linux-gnu
```

**Integration Tests:**
```bash
cd drivers/wifi/aic8800/tests
cargo test --target x86_64-unknown-linux-gnu
```

### Run Specific Tests

```bash
cd drivers/wifi/aic8800/mock
cargo test --target x86_64-unknown-linux-gnu test_firmware_download
```

### Run Tests with Verbose Logging

```bash
cd drivers/wifi/aic8800/mock
RUST_LOG=debug cargo test --target x86_64-unknown-linux-gnu -- --nocapture
```

## Understanding the Mock Utilities

### Mock SDIO Device

Simulates an AIC8800 connected via SDIO:

```rust
use aic8800_mock::MockSdioDevice;

// Create and initialize
let sdio = MockSdioDevice::new();
sdio.initialize()?;
sdio.set_block_size(512)?;

// Read/write operations
sdio.write_byte(0x1000, 0x42)?;
let value = sdio.read_byte(0x1000)?;

// Firmware download
let firmware = vec![0xAA; 4096];
sdio.download_firmware(0x00100000, &firmware)?;

// Verify firmware
assert!(sdio.verify_firmware_at(0x00100000, &firmware));

// Check transaction history
let transactions = sdio.get_transactions();
```

### Mock Firmware Loader

Manages firmware files for testing:

```rust
use aic8800_mock::MockFirmwareLoader;

// Create loader
let loader = MockFirmwareLoader::new();

// Add firmware
let fw = MockFirmwareLoader::create_aic8800_firmware();
loader.add_firmware("fmacfw_8800d80.bin", fw);

// Load firmware
let loaded = loader.load_firmware("fmacfw_8800d80.bin")?;
```

### Register Definitions

Access hardware register definitions:

```rust
use aic8800_mock::Aic8800Registers;

// Register addresses
let chip_id_reg = Aic8800Registers::REG_CHIP_ID;
let ctrl_reg = Aic8800Registers::REG_HOST_CTRL;

// Control bits
let reset = Aic8800Registers::HOST_CTRL_RESET;
let enable = Aic8800Registers::HOST_CTRL_ENABLE;

// Helper functions
if Aic8800Registers::is_valid_chip_id(chip_id) {
    println!("Chip: {}", 
             Aic8800Registers::chip_id_to_string(chip_id));
}
```

## Project Structure

```
drivers/wifi/aic8800/
├── run_tests.sh          # Run all tests
├── README.md             # Full documentation
├── QUICKSTART.md         # This file
├── IMPLEMENTATION_STATUS.md  # Implementation details
│
├── aic8800.h             # C++ driver header
├── aic8800.cc            # C++ driver implementation
├── BUILD.gn              # Build configuration
│
├── mock/                 # Rust mock utilities
│   └── src/
│       ├── sdio.rs       # Mock SDIO device
│       ├── firmware.rs   # Mock firmware loader
│       └── register.rs   # Register definitions
│
├── tests/                # Rust integration tests
│   └── integration_test.rs
│
├── linux_reference/      # Linux driver mapping
│   └── README.md
│
└── firmware/             # Firmware binaries
    └── aic8800DC/
```

## Development Workflow

### Adding New Tests

1. Add test to `mock/src/*.rs` (for unit tests) or `tests/integration_test.rs` (for integration tests)
2. Run tests: `./run_tests.sh`
3. Verify all tests pass

Example unit test:
```rust
#[test]
fn test_my_feature() {
    let sdio = MockSdioDevice::new();
    sdio.initialize().unwrap();
    // ... test code ...
    assert!(condition);
}
```

Example integration test:
```rust
#[test]
fn test_my_integration() {
    let sdio = MockSdioDevice::new();
    let loader = MockFirmwareLoader::new();
    // ... test complete flow ...
}
```

### Adding New Registers

1. Add constant to `mock/src/register.rs`:
   ```rust
   pub const REG_MY_NEW_REG: u32 = 0x00000020;
   ```

2. Add corresponding constant to C++ header `aic8800.h`:
   ```cpp
   static constexpr uint32_t kRegMyNewReg = 0x00000020;
   ```

3. Update documentation in `linux_reference/README.md`

### Debugging Tests

Enable verbose logging:
```bash
RUST_LOG=trace cargo test --target x86_64-unknown-linux-gnu -- --nocapture
```

Print transaction history:
```rust
let transactions = sdio.get_transactions();
for tx in transactions {
    println!("Addr: 0x{:08x}, Write: {}, Data: {:02x?}",
             tx.address, tx.is_write, tx.data);
}
```

Dump memory contents:
```rust
let memory = sdio.get_memory_snapshot();
for (addr, value) in memory.iter() {
    println!("0x{:08x}: 0x{:02x}", addr, value);
}
```

## Common Issues

### Cargo Not Found
If you get "cargo: command not found":
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -y
source "$HOME/.cargo/env"
```

### Wrong Target Error
If you get "can't find crate for `core`":
```bash
# The tests automatically use x86_64-unknown-linux-gnu target
# via .cargo/config.toml in each test directory
```

### Test Failure
If a test fails:
1. Run with verbose output: `RUST_LOG=debug cargo test ... -- --nocapture`
2. Check the test output for assertion failures
3. Review the mock implementation
4. Check register addresses match between Rust and C++

## Next Steps

1. ✅ Run tests: `./run_tests.sh`
2. ✅ Read full documentation: `README.md`
3. ✅ Understand Linux mapping: `linux_reference/README.md`
4. ⏭️ Build C++ driver (requires Fuchsia SDK)
5. ⏭️ Test on hardware (requires A527 board)

## Documentation

- **README.md** - Complete driver documentation
- **linux_reference/README.md** - Linux↔Fuchsia mapping
- **IMPLEMENTATION_STATUS.md** - Implementation details and status
- **QUICKSTART.md** - This file

## Support

For questions or issues:
1. Check the documentation files listed above
2. Review test examples in `mock/src/*.rs` and `tests/integration_test.rs`
3. Check Linux driver reference in `vendor/aic8800-linux/`

## Test Coverage

Current test coverage:

**Mock Utilities (19 tests):**
- SDIO: initialization, block size, byte ops, multi-block, firmware download, error handling
- Firmware: loading, multiple files, load counting, test generation
- Registers: chip ID validation, status checks, bit operations

**Integration Tests (10 tests):**
- Full initialization flow
- Register sequences
- Firmware download and verification
- Error recovery
- Transaction history
- Block transfers
- Interrupt handling simulation
- Status transitions
- MAC address operations
- Multiple firmware handling

**Total: 29 tests, all passing ✅**
