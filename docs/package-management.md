## Manifest Format

A Vinglish package is described by `ving.toml`. The manifest contains the package name, its version, and the dependency list. A minimal example is:

```toml
[package]
name = "demo-app"
version = "0.1.0"
edition = "2024"

[dependencies]
core = "^0.3.0"
```

`package.name` identifies the package. `package.version` follows semantic versioning. The edition field states the target compatibility level for syntax and library behavior. The dependency block can contain exact versions, semver ranges, Git links, or local paths.

## Dependency Specification

The package manager supports several dependency forms. A simple range is:

```toml
[dependencies]
core = "^0.3.0"
```

Git dependencies are supported as metadata entries. Local path dependencies are useful for monorepos and local testing. The registry index can be overridden with `VINGLISH_REGISTRY_INDEX` for offline verification or a local mirror.

## Lock File

`ving.lock` records the exact versions selected for a build. It exists to prevent dependency drift. When a build resolves to a specific version, that version should remain pinned unless the user intentionally updates the lock output. The package manager validates the lock file against the manifest requirements so a mismatch is reported early instead of allowing an inconsistent build.

## Adding Dependencies

The standard workflow is:

```sh
vng pkg init
vng pkg add core
```

This creates the manifest and resolves a dependency from the configured registry. A dependency can also be added from a Git URL or a local path. The package manager updates both the manifest and the lock file when the resolution succeeds.

## Local Development and Publishing

Local development is easiest with path dependencies or a local registry index. That keeps builds deterministic and makes internal testing easier before a package is published. The intended public workflow includes registry publication through a command such as `vng pkg publish`, with version increments managed through the semver field in the manifest.

## Registry Index

The registry is a JSON document that exposes package names, versions, and source metadata. In the repository, `registry/index.json` is an example index. The resolver selects the newest version that satisfies the requirement and then checks that the locked version still matches the dependency range. This is the difference between a simple package lookup and a semver-aware package manager.
