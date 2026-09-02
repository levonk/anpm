# apmw-core

Reusable package manager detection, ecosystem mapping, and version resolution
for the [apmw](https://github.com/levonk/apmw) project.

## Overview

`apmw-core` provides the core detection, ecosystem mapping, and version
resolution logic extracted from the `apmw` binary. It is designed to be
reusable by downstream projects that need package-manager detection without
the full CLI/daemon surface.

## Features

- **Package manager detection** — detects the correct package manager for a
  given project directory (npm, pnpm, yarn, pip, uv, cargo, go, and more).
- **Ecosystem mapping** — maps within-ecosystem alternatives to canonical
  runners (e.g. pip to uv within Python, npm/yarn/bun to pnpm within Node).
- **Version resolution** — resolves versions intelligently with a default
  minimum-release-age supply-chain defense.
- **Workspace detection** — detects monorepo/workspace roots and member
  packages.
- **Version info** — structured version information for the running tool.

## Usage

Add `apmw-core` to your `Cargo.toml`:

```toml
[dependencies]
apmw-core = "0.1"
```

```rust
use apmw_core::detect;

let detected = detect::detect_package_manager(".")?;
println!("{:?}", detected);
```

## License

MIT
