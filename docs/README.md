# Soliloquy Docs

Current docs cover the active Cargo, Bun, Servo/RV8, and desktop paths.

Browser-engine work lives in `atechnology-company/rv8` (`../rv8` locally). Soliloquy no longer carries an active in-repo RV8 crate.

Alpenglow owns the installable OS, kernel, rootfs, `sold` bridge, and service graph in `../alpenglow`. Soliloquy owns the Svelte desktop environment and browser chrome.

The macOS desktop path uses Crepuscularity GPUI chrome with Servo's built-in chrome disabled. Its smoke check is dry-run only; the real launcher can start or reuse `sold` for local runtime APIs, but it must not load the Svelte appliance chrome. See [Browser Chrome](./browser_chrome.md).

## Core

- [Index](./INDEX.md)
- [Build](./build.md)
- [Contributing](./contributing.md)
- [API Contract](./api_contract.md)
- [Browser Chrome](./browser_chrome.md)
- [RV8 Linkage Roadmap](./rv8_linkage_roadmap.md)
- [Testing](./testing/README.md)
