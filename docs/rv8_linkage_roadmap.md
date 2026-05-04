# RV8 Linkage Roadmap

This document tracks the remaining work to move Soliloquy from the current hybrid `mozjs` / `v8-experimental` state to a real Servo + V8 runtime linkage.

## Current State

- Servo has an in-tree backend selection seam controlled by `SOLILOQUY_JS_ENGINE`.
- `v8-experimental` is real as a mode selection, but Servo script still boots through `mozjs`.
- `servoshell` is already patched in-tree and connected to Soliloquy optimization hooks.
- Phase 1 of the bridge extraction has started: live DOM snapshot storage and typed bridge operations now live in `third_party/servo/components/servo/soliloquy_bridge.rs`.
- `WebView::evaluate_javascript()` has a Soliloquy dispatcher for:
  - simple literals and `+` expressions
  - structured command calls through `window.__soliloquyEval(...)`
  - live snapshot-backed reads for `document.title`, `location.href`, and `document.readyState`
  - snapshot-backed writes for `document.title` and absolute / relative `location.href`
  - stable command result envelopes for ok, error, and unsupported outcomes
- The `v8-experimental` dispatch backend now owns an explicit dispatch-only V8 isolate owner stub so the status surface can distinguish bridge dispatch from a real isolate.
- Servo now publishes the bridge command schema from `soliloquy_bridge_schema.json`; the shell-side V8 mock includes the same file when reporting bridge capabilities.
- Servo now has an opt-in `soliloquy_v8` feature that wires the V8 owner stub to `rusty_v8` platform and thread-local isolate initialization while leaving the default dispatch-only path unchanged.
- The shell-side V8 mock now understands the same typed bridge command surface and keeps a small DOM snapshot in sync with shell navigations.
- Unsupported evaluation paths still fall back to Servo's existing `mozjs` path.

## What Has Been Done

- Added a typed Servo-side bridge module for the current snapshot-backed operations.
- Replaced direct snapshot ownership in the evaluator with bridge helpers and typed read/write targets.
- Added a stable command result envelope carrying `ok`, `status`, `value`, and `detail`.
- Replaced the structured command dispatcher internals with typed command variants before execution.
- Moved the Servo-side bridge command enum and command-target parsing into the bridge boundary.
- Added a narrow `location.href` write path that updates the live snapshot and marks `document.readyState` as `loading`.
- Resolved relative `location.href` writes against the live document URL before mutation routing.
- Routed validated `location.href` writes through Servo's `LoadUrl` constellation message.
- Taught the shell-side V8 mock to execute typed bridge reads, writes, command envelopes, and navigation snapshot updates.
- Added the initial Servo-side script backend trait with `mozjs` fallback and `v8-experimental` dispatch implementations.
- Added a dispatch-only V8 isolate owner stub behind the `v8-experimental` backend and exposed it through `engine.status`.
- Added a shared bridge schema JSON and exposed it through `dom.capabilities` on both the Servo bridge and shell V8 mock.
- Added an opt-in `soliloquy_v8` feature that initializes a real `rusty_v8` platform and thread-local isolate for the V8 owner status path.
- Fixed the Servo bridge test compile path against Servo's composite `WebViewId`, Rust 2024 environment mutation rules, and the `HistoryChanged` WebView ownership path.
- Kept the `rv8` dispatcher local-first, with fallback to Servo's existing `mozjs` path for unsupported operations.
- Kept the shell-side JS engine status plumbing aligned with `SOLILOQUY_JS_ENGINE`.
- Upgraded the desktop UI dependency stack and cleared the open `ui/desktop` Dependabot alerts locally.
- Verified the current shell and UI paths with passing tests/builds.
- Verified Servo-side bridge tests with explicit macOS SDK sysroot:
  - `cargo test -p servo soliloquy_javascript --lib`
  - `cargo test -p servo javascript_evaluator --lib`
  - `cargo test -p servo soliloquy_bridge --lib`
  - `cargo test -p servo soliloquy_javascript --lib --features soliloquy_v8`
- Added `tools/rv8_servo_test.sh` so the targeted Servo bridge checks can run with the required macOS SDK sysroot env without hand-typing it.
- Added schema-driven parser tests for `soliloquy_bridge_schema.json` so declared commands and DOM targets must parse through the typed Servo bridge boundary.

## What Is Still Missing

- A real V8 isolate running inside Servo-owned evaluation paths.
- A typed host bridge between Servo script operations and the Soliloquy V8 runtime.
- DOM object identity, handle lifetime, and cross-runtime serialization rules.
- Mutation routing from V8-owned calls into the actual Servo script / DOM thread.
- Event delivery from Servo into the V8 side for navigation, load, timers, and DOM changes.
- A plan to remove or sharply reduce `mozjs` ownership without breaking Web IDL bindings.

## Compositor Reference Notes

Hyprland is a useful reference for the RV8 compositor shape, not a code source to copy. Its optimized path centers on per-output frame scheduling, a damage ring retained across the swap chain depth, explicit damage entry points, a pass queue that is built before GPU execution, and visibility / occlusion checks before expensive rendering. See:

- Hyprland rendering pipeline notes: <https://deepwiki.com/hyprwm/Hyprland/6.1-rendering-pipeline>
- Hyprland repository: <https://github.com/hyprwm/Hyprland>

RV8 should mirror those architecture choices in Rust / WGPU terms:

- Frame production must be damage-driven; avoid a permanent 16 ms render loop when no visible content changed.
- Track damage per output / surface and retain recent damage across the buffer queue so partial redraw is safe after swaps.
- Build an explicit render pass list from tabs, layers, popups, cursor, overlays, and browser UI before submitting GPU commands.
- Cull invisible or fully occluded layers before pass construction.
- Keep presentation timing and frame statistics in the compositor boundary so QEMU and hardware tests can assert real frame flow.
- Add snapshot textures for closing / animating tabs or surfaces so animations do not depend on a live renderer buffer.

## Recommended Phases

### Phase 1: Harden The Host Bridge

- Completed in part:
  - moved the current snapshot store behind a dedicated bridge module
  - introduced typed read targets for `document.title`, `location.href`, and `document.readyState`
  - introduced typed write targets for `document.title` and `location.href`
  - defined the initial stable command result envelope for ok, error, and unsupported operations
  - moved structured command dispatch through typed command variants
  - moved Servo-side bridge command definitions into the bridge boundary
  - connected absolute `location.href` writes to Servo's real navigation request path
  - resolved relative `location.href` writes against the live document URL
  - mirrored the typed bridge surface in the shell-side V8 mock
  - published the bridge command schema from Servo and included it from the shell-side V8 mock
  - added schema-driven parser tests for declared commands, read targets, and write targets
- Still to do:
  - add more result-envelope coverage for arrays and objects as bridge commands expand

### Phase 2: Introduce A Real V8 Execution Core

- Completed in part:
  - added a Servo-side trait for script execution backends
  - added a `mozjs` fallback implementation that declines local execution
  - moved the current Soliloquy V8 dispatch path behind a `v8-experimental` backend implementation
  - added a dispatch-only V8 isolate owner stub so backend status can report `isolateOwner` and `realIsolate`
  - added an opt-in `soliloquy_v8` feature that bootstraps `rusty_v8` platform / thread-local isolate ownership
- Still to do:
  - define isolate lifetime, value transport, and error propagation contracts
  - route only the narrow bridge operations through V8 first
  - keep direct DOM execution out of scope until value transport, error propagation, and isolate lifetime are stable

### Phase 3: Add DOM Handle Semantics

- Introduce opaque node / window / document handles instead of string-only operations.
- Define lookup, invalidation, and stale-handle behavior.
- Add explicit thread-affinity rules for script thread access.

### Phase 4: Expand Mutation Coverage

- Add write operations beyond title:
  - location navigation
  - selected attribute / property mutation
  - limited element creation and text updates
- Each added operation needs:
  - bridge command
  - serializer contract
  - unit tests
  - fallback behavior

### Phase 5: Move Evaluation Ownership

- Route a bounded subset of `evaluate_javascript()` entirely through the V8 backend.
- Add feature-gated telemetry for:
  - backend chosen
  - fallback rate
  - unsupported operation rate
  - serialization failures
- Only after fallback rates are low should wider JS execution migrate away from `mozjs`.

### Phase 6: Rework Script Bindings

- Audit the current `js::...` coupling in Servo script and bindings.
- Identify the smallest binding surface that can be abstracted first.
- Expect this to be the longest and highest-risk phase; Web IDL integration is where a full engine swap becomes real work instead of dispatcher plumbing.

## Acceptance Criteria For Full Linkage

- Servo can boot with V8 selected and execute real page script through V8, not only host-dispatched probes.
- DOM reads and writes go through a typed bridge or native binding layer rather than snapshot-only helpers.
- Fallback to `mozjs` is either removed or confined to explicitly unsupported legacy paths.
- Integration tests cover:
  - evaluation
  - DOM mutation
  - navigation
  - event delivery
  - isolate teardown
  - multi-webview separation
- QEMU boot tests confirm visible browser rendering and successful script execution under the selected V8 mode.

## Current Blockers

- Plain local Servo Rust validation still needs a toolchain env shim on this machine.
  - Running without an explicit Xcode SDK sysroot still fails in `mozangle v0.5.5` while generating shader bindings against Homebrew LLVM 21 libc++ headers.
  - `tools/rv8_servo_test.sh bridge` applies the env shim and runs the non-feature Servo bridge test groups.
  - `tools/rv8_servo_test.sh v8` applies the env shim and runs the feature-gated `soliloquy_v8` bridge test group.
  - The feature-gated `soliloquy_v8` test also needs the `rusty_v8` static archive available; the escalated run downloaded/built it and passed.
- The current bridge is intentionally narrow and does not yet own general DOM execution.
- The default branch still shows the historical Dependabot alerts until the `rv8` dependency updates are merged.

## Immediate Next Steps

1. Broaden the navigation bridge test into an integration path that crosses `WebView::evaluate_javascript()` and observes the emitted `LoadUrl` request.
2. Add the first real V8 evaluation call behind the thread-local owner, limited to literal / arithmetic smoke tests before DOM transport.
3. Decide whether the Servo SDK sysroot shim belongs in CI, local docs only, or a target-specific upstream Servo build configuration.
4. Extend schema coverage to mutation definitions once the next mutation command is added.
5. Continue replacing the RV8 compositor stub with a Hyprland-style damage-driven scheduler, damage ring, and explicit render-pass queue.
