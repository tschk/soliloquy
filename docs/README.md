# Soliloquy Docs

Current docs cover the active Cargo, Bun, Alpine, Servo/RV8, and `sold` paths.

RV8 browser-engine work has a standalone local sibling repo at `../rv8`; Soliloquy still keeps the in-tree `src/rv8` crate active until a remote Git submodule or dependency replaces it.

The desktop browser shell owns the visible browser chrome. Servo is launched with its browser chrome disabled for the appliance path so `os://terminal`, tabs, modes, and navigation controls do not double up.

Browser chrome is authored from a Crepuscularity template and shared into the web desktop through `ui/desktop/src/lib/crepuscularity/browserChrome.ts`; see [Browser Chrome](./browser_chrome.md).

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
