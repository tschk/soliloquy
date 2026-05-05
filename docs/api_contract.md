# Soliloquy / Rover Contract

This repo now treats `rover-desktop` as the host boundary for engine access.

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

- `rover`: engine loop, scheduler, tab/session model, service contracts, optional FFI
- `rover-desktop`: host windowing, compositor/input integration, control plane
- `soliloquy`: appliance runtime, policy layer, local tools, and Alpine packaging
