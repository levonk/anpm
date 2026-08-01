# Detecting a Package Manager

This example shows how apmw detects the package manager for the current
project.

## Auto-detection

```bash
$ cd my-python-project
$ apmw detect
```

apmw examines the project directory for lockfiles and manifests
(`pyproject.toml`, `requirements.txt`, `uv.lock`, `package.json`,
`pnpm-lock.yaml`, `Cargo.toml`, `go.mod`, `Gemfile`, etc.) and reports the
detected manager(s) with confidence scores.

## Human-readable output

```bash
$ apmw detect --human
```

By default, apmw emits TOON (token-optimized) output for AI agents. Use
`--human` for human-readable output.

## JSON output

```bash
$ apmw detect --json
```

## Force a specific manager

```bash
$ apmw detect --manager cargo
```

Skips auto-detection and reports the forced manager. This is useful in CI
or when you know the manager and want to skip detection overhead.

## Select specific fields (AXI mode)

```bash
$ apmw detect --fields manager,ecosystem
```

Emits only the requested fields (token-optimized for AI agents).

## No-args behavior

Running `apmw detect` with no arguments detects the current project — this
is the content-first no-args behavior from ADR-20260607001.
