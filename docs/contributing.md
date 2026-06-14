# Contributing

Use current repo paths only: Cargo, Bun, Servo/RV8 bridge scripts, and desktop UI scripts.

## Gates

```bash
cargo fmt --check
cargo test -p soliloquy-shell --lib
cd ui/desktop && bun run check && bun run build
```

## Package Manager

Use `wax` for system packages. Use `bun` for JavaScript. Do not add telemetry or secret-bearing config.

## License

First-party code is MPL-2.0.
