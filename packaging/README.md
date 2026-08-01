# apmw packaging generators

Multi-ecosystem packaging generators for apmw. Each generator produces a
package definition that wraps the **pre-built Rust binary** — the binary is
the single source of truth, and each ecosystem package downloads it from the
GitHub release.

## Ecosystems

| Directory  | Ecosystem       | Output                              | Package name |
|------------|-----------------|-------------------------------------|--------------|
| `npm/`     | npm / pnpm      | `package.json` + `install.js`       | `apmw`       |
| `pypi/`    | PyPI / uv / pip | `pyproject.toml` + Python module     | `apmw`       |
| `brew/`    | Homebrew        | `apmw.rb` formula                    | `apmw`       |
| `nix/`     | Nix             | `default.nix` + `flake.nix`          | `apmw`       |
| `devbox/`  | Devbox          | `devbox.json`                        | `apmw`       |
| `apt/`     | apt (source)    | `debian/` source package             | `apmw`       |
| `winget/`  | Winget          | YAML manifests                       | `levonk.apmw`|
| `apk/`     | Alpine          | `APKBUILD`                           | `apmw`       |
| `deb/`     | Debian (binary) | `.deb` layout                        | `apmw`       |
| `rpm/`     | RPM / Fedora    | `apmw.spec`                          | `apmw`       |
| `aur/`     | Arch AUR        | `PKGBUILD` + `.SRCINFO`              | `apmw-bin`   |

> **AUR naming**: The AUR package uses `apmw-bin` to avoid collision with the
> unrelated AUR package `apmw` (an apt-to-pacman translator). See PRD Open
> Question 2.

## Usage

Each generator accepts a version and optional output directory:

```bash
# Generate all packages for version 0.1.0
for gen in npm pypi brew nix devbox apt winget apk deb rpm aur; do
    packaging/$gen/generate.sh 0.1.0 ./dist/$gen
done
```

The `deb/` generator also takes a target triple as the second argument:

```bash
packaging/deb/generate.sh 0.1.0 x86_64-unknown-linux-gnu ./dist/deb
```

## Release pipeline

The [`.github/workflows/release.yml`](../.github/workflows/release.yml)
workflow is triggered on tag push (`v*`). It:

1. Builds the Rust binary for 5 targets (x86_64/aarch64 Linux, x86_64/aarch64
   macOS, x86_64 Windows).
2. Creates a GitHub Release with all binary archives.
3. Runs all ecosystem generators.
4. Publishes to registries (npm, PyPI, Homebrew tap, AUR, Winget) — each
   publish step is conditional on the corresponding secret being set, so
   missing secrets do not break the build.

## Shared metadata

[`metadata.sh`](metadata.sh) contains shared project metadata (name, version,
license, repository URL) and helper functions used by all generators.
