# Adding a Package

This example shows how to add (install) packages with apmw, including
runtime vs. development dependencies and manager overrides.

## Install a runtime dependency

```bash
$ cd my-node-project
$ apmw install express
Installed express via pnpm
```

apmw auto-detects the package manager (pnpm, based on `pnpm-lock.yaml`),
resolves the version (respecting the 2-day minimum-release-age default),
scans for security issues, and installs.

## Install a development dependency

```bash
$ apmw install jest --dev
Installed jest (dev) via pnpm
```

The `--dev` flag marks the dependency as development/build-time. This is
recorded in the audit log and telemetry.

## Force a specific manager

```bash
$ apmw install lodash --manager npm
Installed lodash via npm
```

The `--manager` (or `--use`) flag overrides auto-detection. Valid managers
include: `pnpm`, `npm`, `yarn`, `bun`, `uv`, `pip`, `poetry`, `cargo`,
`go`, `gem`, `brew`, `nix`, `devbox`, `apt`, `docker`, and more (see
`apmw --help`).

## Dry run

```bash
$ apmw install react --dry-run
```

Shows what would happen without making any changes.

## JSON output (for scripting)

```bash
$ apmw install express --json
```

Emits the result as JSON for programmatic consumption.

## Skip security scanning

```bash
$ apmw install express --no-scan
```

Skips the security scan (not recommended for untrusted packages).

## Scan only (don't install)

```bash
$ apmw install express --scan-only
```

Runs the security scan and exits without installing.

## Audit log

Every install is recorded in the audit log:

```bash
$ apmw audit-log
```
