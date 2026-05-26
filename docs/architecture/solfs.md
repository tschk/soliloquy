# SolFS

SolFS is the Soliloquy root filesystem for internet-appliance images. Its job is narrower than ext4, Btrfs, XFS, EROFS, or SquashFS: boot a verified browser operating system quickly, expose immutable system generations, and keep mutable browser state out of the root image.

## Why SolFS Exists

General filesystems optimize for broad POSIX workloads. Soliloquy has a smaller contract:

- `/usr`, `/etc`, OpenRC service graphs, UI bundles, Servo launchers, `sold`, and policy files are immutable generation content.
- Browser cache is disposable and should not be treated like durable user state.
- Browser profiles, downloads, terminal state, and Wax state are explicit persistent mounts outside the root image.
- OS rollback should be a filesystem generation operation rather than a package-manager repair operation.
- Image verification should be native to the root format instead of bolted on after mount.

SolFS v0 therefore uses a compact read-only image with a fixed header, fixed entries, a packed name table, aligned file payloads, and SHA-256 content digests. The first kernel slice mounts only valid SolFS images and exposes an immutable root. Userspace tooling creates and inspects the image format so the on-disk contract is testable before the kernel grows full page-cache-backed file reads.

## Kernel Boundary

The kernel driver is Rust-first with a small C VFS shim:

- C owns `register_filesystem`, `mount_bdev`, `super_block`, root inode creation, and Linux VFS ABI details.
- Rust owns SolFS format validation and will own metadata lookup, digest policy, and read planning as the Rust side grows.
- The C shim is replaceable when upstream Rust VFS filesystem abstractions cover the required mount, inode, dentry, directory, and page-cache operations.

V is not used inside the kernel. V remains suitable for generated manifests and userspace policy helpers, but generated C is not a good Linux-kernel boundary because the kernel is a constrained environment rather than a normal libc target.

## Format V0

Header:

- magic: `SOLFSV01`
- version: `1`
- entry count
- entries offset
- names offset
- data offset
- image size
- flags

Entry:

- inode
- parent inode
- name offset and length
- kind: directory or file
- mode, uid, gid
- file data offset and size
- SHA-256 digest

The root entry is inode `1`, parent `1`, directory kind, and empty name. Symlinks are rejected in v0 so the immutable root cannot smuggle host-dependent path behavior into the boot image.
