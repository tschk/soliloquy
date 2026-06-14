# Soliloquy Documentation Index

Soliloquy is currently documented around Cargo, Bun, Servo, RV8/V8 bridge work, and the desktop environment. OS install work lives in `../alpenglow`.

## Start Here

- [Project README](../readme.md)
- [Build Guide](./build.md)
- [Contributing](./contributing.md)

## Runtime And Desktop

- [API Contract](./api_contract.md)
- [RV8 Linkage Roadmap](./rv8_linkage_roadmap.md)
- [Browser Optimizations](./browser_optimizations.md)

## Guides

- [Developer Guide](./guides/dev_guide.md)
- [Tools Reference](./guides/tools_reference.md)

## Testing

- [Testing README](./testing/README.md)

## Current Commands

```bash
cargo test -p soliloquy-shell --lib
./tools/rv8_servo_test.sh bridge
./tools/soliloquy/build_ui.sh
./tools/soliloquy/start.sh
```
