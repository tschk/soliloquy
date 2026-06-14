# Soliloquy / Engine Contract

This repo treats RV8 as the browser-engine boundary for page loading, tab state, and runtime integration. Soliloquy owns the desktop shell and browser chrome.

## Services

- `LifecycleService`
  - `create_session()`
  - `open_tab(session, initial_url)`
  - `close_tab(tab)`
- `NavigationService`
  - `navigate(tab, url)`
  - `reload(tab)`
- `DomService`
  - `get_dom_snapshot(tab)`
  - `click(tab, node_id)`
- `DiagnosticsService`
  - `engine_state()`

## Capability / handle types

- `SessionHandle`
- `TabHandle`
- `WindowHandle`
- `CapabilityToken`

## Ownership

- `rv8`: engine loop, scheduler, tab/session model, service contracts, optional FFI
- `ui/desktop`: browser chrome, modes, compositor/input integration, control plane
- `soliloquy`: desktop browser launcher, RV8 bridge, runtime integration, and browser-facing optimization work
- `alpenglow`: installable OS, kernel policy, `sold`, rootfs, service graph, and backend packaging
