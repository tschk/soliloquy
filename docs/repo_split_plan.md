# Repo Split Plan

## Target repos

- `atechnology-company/rv8`
- `atechnology-company/soliloquy`

## Current incremental state

- `../rv8` is now a standalone local Git repository seeded from `src/rv8`.
- Soliloquy still keeps and builds the in-tree `src/rv8` crate so the appliance can use the browser engine without depending on a remote URL.
- The sibling `../rv8` manifest points back to `../soliloquy/src` for the shared optimization crate while that library remains in Soliloquy.
- `../rover` is unrelated to this split and should not be used for RV8 migration work.

## Next migration steps

1. Keep `src/rv8` and `../rv8` synchronized until a remote `rv8` repo exists.
2. Replace the in-tree crate with a Git submodule or path dependency once the remote URL is ready.
3. Move RV8-only Servo/V8 bridge code behind the `rv8` API boundary.
4. Keep Soliloquy-specific appliance, `sold`, and desktop UI code in this repo.
