# RV8 Linkage Roadmap

This document tracks the remaining work to move Soliloquy from the current hybrid `mozjs` / `v8-experimental` state to a real Servo + V8 runtime linkage.

## Current State

- Servo has an in-tree backend selection seam controlled by `SOLILOQUY_JS_ENGINE`.
- `v8-experimental` is real as a mode selection, but Servo script still boots through `mozjs`.
- `servoshell` is already patched in-tree and connected to Soliloquy optimization hooks.
- `WebView::evaluate_javascript()` has a Soliloquy dispatcher for:
  - simple literals and `+` expressions
  - structured command calls through `window.__soliloquyEval(...)`
  - live snapshot-backed reads for `document.title`, `location.href`, and `document.readyState`
  - snapshot-backed writes for `document.title`
- Unsupported evaluation paths still fall back to Servo's existing `mozjs` path.

## What Is Still Missing

- A real V8 isolate running inside Servo-owned evaluation paths.
- A typed host bridge between Servo script operations and the Soliloquy V8 runtime.
- DOM object identity, handle lifetime, and cross-runtime serialization rules.
- Mutation routing from V8-owned calls into the actual Servo script / DOM thread.
- Event delivery from Servo into the V8 side for navigation, load, timers, and DOM changes.
- A plan to remove or sharply reduce `mozjs` ownership without breaking Web IDL bindings.

## Recommended Phases

### Phase 1: Harden The Host Bridge

- Replace ad hoc string commands with typed bridge operations:
  - `GetDocumentTitle`
  - `SetDocumentTitle`
  - `GetLocationHref`
  - `GetReadyState`
- Move the current snapshot store behind a dedicated bridge module instead of keeping it inside the evaluator file.
- Define a stable result envelope for strings, booleans, null, errors, and unsupported operations.

### Phase 2: Introduce A Real V8 Execution Core

- Add a Servo-side trait for script execution backends with one implementation for `mozjs` and one for Soliloquy V8.
- Route only the narrow bridge operations through V8 first.
- Keep direct DOM execution out of scope until value transport, error propagation, and isolate lifetime are stable.

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

- Local Servo Rust validation is still blocked on the existing `mozangle` Apple / LLVM header failure on this machine.
- The current bridge is intentionally narrow and does not yet own general DOM execution.
- GitHub alert cleanup for `ui/desktop` needs to be pushed before Dependabot clears the open alerts on the repo.

## Immediate Next Steps

1. Extract the snapshot-backed operations into a dedicated `rv8` bridge module inside Servo.
2. Replace string command names with typed bridge enums.
3. Teach the shell-side V8 layer to execute those bridge operations instead of only reporting mock status.
4. Add one end-to-end mutation test around `document.title` using the typed bridge.
5. Fix the local `mozangle` toolchain issue so Servo-side unit tests can run in this environment.
