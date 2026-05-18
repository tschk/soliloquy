# Soliloquy Docs

Current docs cover the active Cargo, Bun, Alpine, Servo/RV8, and `sold` paths.

Browser-engine work lives in `atechnology-company/rv8` (`../rv8` locally, seeded from `src/rv8`). Soliloquy still keeps the in-tree `src/rv8` crate active until a submodule or path dependency replaces it. See [Repo Split Plan](./repo_split_plan.md).

The Alpine appliance owns the Svelte browser chrome and `sold` bridge. Servo is launched with its browser chrome disabled there so `os://terminal`, tabs, modes, and navigation controls do not double up.

The macOS desktop path is browser-only Servo with its built-in chrome disabled. It must not start `sold` or load the Svelte appliance chrome; see [Browser Chrome](./browser_chrome.md).

## Core

- [Index](./INDEX.md)
- [Build](./build.md)
- [Contributing](./contributing.md)
- [V0 Architecture](./v0-architecture.md)
- [Appliance System](./architecture/appliance-system.md)
- [API Contract](./api_contract.md)
- [Browser Chrome](./browser_chrome.md)
- [RV8 Linkage Roadmap](./rv8_linkage_roadmap.md)
- [Testing](./testing/README.md)
