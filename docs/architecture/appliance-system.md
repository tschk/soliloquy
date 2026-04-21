# Appliance System Architecture

## Goals

- Keep the base system immutable.
- Restrict user writes to home directories only.
- Keep browser profiles system-managed.
- Keep cross-device sync optional and separate from the immutable base image.
- Stay minimal in the base image while remaining feature rich through optional services and plugins.

## Filesystem Layout

- `/` is an immutable system image.
- `/home/<user>` is the only user-writable persistent area.
- `/var/lib/soliloquy/browser/profiles` stores system-managed browser profiles.
- `/var/lib/soliloquy/system` stores system-owned plugin and policy state.
- `/var/cache/soliloquy` stores system-owned cache data.
- `/var/log/soliloquy` stores system-owned logs.
- `/tmp` is a system scratch area, not a user workspace.

## Browser Data Model

- Browser profile data is not stored as user-managed dotfiles.
- The browser reads and writes explicit system-owned directories.
- User files remain in `/home/<user>`, but history, tabs, cookies, sync metadata, and caches stay under system control.

## Plugin Model

- Plugins are optional downloads rather than part of the immutable base.
- Plugin policy defaults live in `/etc/soliloquy/system.json`.
- Writable plugin state lives in `/var/lib/soliloquy/system/plugin-state.json`.
- The first plugin is `remote-sync`, with separate feature flags for:
  - file sync
  - photo sync
  - clipboard sync

## Service Architecture

The target service architecture borrows from several operating system designs:

- Solaris: declarative service graph, restart behavior, maintenance states, boot environments.
- GNU Hurd: path-backed service views and translator-style namespace composition.
- Fuchsia/Zircon: capability-driven service boundaries, driver isolation, structured diagnostics.
- BSD: simple service supervision, `rc` discipline, `devfs`-style device handling, log rotation.
- Redox and Plan 9: treat devices and services as named resources instead of hard-coded global subsystems.
- MINIX 3: restartable critical services.
- Haiku: strong separation between immutable system state and user state.
- ToaruOS and Vinix: lean startup path, explicit boot sequencing, minimal runtime assumptions.

## Immediate System Changes

These changes are already reflected or scaffolded in the current appliance path:

- Immutable-root assumptions are encoded in the system policy file.
- Browser storage defaults point at system-owned directories.
- The local system service exposes policy and plugin state over authenticated API endpoints.
- Boot scripts create and lock down system runtime directories.
- Plugin state is separated into immutable defaults and writable runtime state.
- System service accounts partition runtime ownership between browser and local system services.

## Near-Term Follow-Up

- Reduce the remaining root-only session path once the display stack can run under a dedicated account.
- Implement plugin download/install state and signature verification.
- Add a generic encrypted sync service and trust model if cross-device sync is enabled.
- Replace ad hoc OpenRC service behavior with a more declarative service registry over time.

## Why These Changes

The operating system research pushed the design in a consistent direction:

- keep the base image boring and immutable,
- move mutable complexity into narrow, auditable services,
- keep device and service policy explicit,
- design for rollback, observability, and recovery from the start.
