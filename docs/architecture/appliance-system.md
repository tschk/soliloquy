# Appliance System Architecture

## Goals

- Keep the base system immutable.
- Restrict user writes to home directories only.
- Keep browser profiles system-managed.
- Make the phone the source of truth when the optional sync plugin is enabled.
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
- The first plugin is `phone-sync`, with separate feature flags for:
  - file sync
  - photo sync
  - clipboard sync

## Phone-Authoritative Sync

- The phone is the trust anchor for profile state.
- Desktop devices can cache and propose state changes, but policy and durable profile truth come from the phone plugin flow.
- The system should eventually expose scoped profile views rather than raw profile directories.

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

## Near-Term Follow-Up

- Add a dedicated system service account for browser/profile ownership instead of using `root`.
- Split `/var/lib/soliloquy/system` and `/var/lib/soliloquy/browser` permissions by service.
- Implement plugin download/install state and signature verification.
- Add a phone-pairing service and encrypted sync log.
- Replace ad hoc OpenRC service behavior with a more declarative service registry over time.

## Why These Changes

The operating system research pushed the design in a consistent direction:

- keep the base image boring and immutable,
- move mutable complexity into narrow, auditable services,
- keep device and service policy explicit,
- design for rollback, observability, and recovery from the start.
