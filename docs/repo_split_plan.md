# Repo Split Plan

## Target repos

- `atechnology-company/rover`
- `atechnology-company/rover-desktop`
- `atechnology-company/soliloquy`

## Current incremental state

- `src/rv8` history was preserved in local branch `codex/rover-rv8-history` using `git subtree split`.
- Engine-oriented code has been physically extracted into the local `../rover` repo under `crates/rover-core/legacy/`.
- The stable API boundary lives in `rover-proto`, with `rover-core` and the `rover-desktop` app depending on it.
- This repo now declares a dependency on `rover-desktop` via local git URL so product work can move to the desktop-app boundary incrementally.

## Next migration steps

1. Replace direct `src/shell/*` engine calls with `rover-desktop` app/control-plane calls.
2. Move desktop-specific Rust binaries and windowing code from `src/shell` into `rover-desktop`.
3. Reduce `rover-core/legacy/` by promoting migrated subsystems behind the stable API.
4. When remote repos are ready, push the extracted repos and replace local git URLs with GitHub URLs.
