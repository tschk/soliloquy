# Repo Split Plan

## Target repos

- `atechnology-company/soliloquy` — appliance runtime, Alpine packaging, `sold`, and product policy
- `atechnology-company/rv8` — canonical browser engine (multi-process IPC, Servo embed, V8, storage)

Archived (do not use for new work): `atechnology-company/rover`, `atechnology-company/rover-desktop`.

A different macOS launcher may also use the folder name `rover` (for example `semitechnological/rover`). That launcher is unrelated to RV8.

## Current incremental state

- `../rv8` is a standalone local Git repository seeded from `src/rv8`.
- Soliloquy still keeps and builds the in-tree `src/rv8` crate so the appliance can use the browser engine without depending on a remote URL.
- The sibling `../rv8` manifest points back to `../soliloquy/src` for the shared optimization crate while that library remains in Soliloquy.

## Next migration steps

1. Keep `src/rv8` and `../rv8` synchronized.
2. Replace the in-tree crate with a Git submodule or path dependency on `atechnology-company/rv8` when ready.
3. Move RV8-only Servo/V8 bridge code behind the `rv8` API boundary.
4. Keep Soliloquy-specific appliance, `sold`, and Alpine UI code in this repo.
