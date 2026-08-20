# Scanning for Vulnerabilities

This example shows how to use apmw's security scanning features.

## Scan a specific package

```bash
$ apmw scan left-pad
Scanning left-pad...
```

Scans the named package for known vulnerabilities.

## Scan the current project

```bash
$ apmw scan
Scanning current project...
```

With no package argument, apmw scans the current project's dependencies.

## Update the vulnerability database first

```bash
$ apmw scan --update-security-db
```

Refreshes the local vulnerability database before scanning.

## Scan during install

apmw scans automatically before installing (two-phase: scan all, then
install all). Control this with:

```bash
# Skip scanning entirely
apmw install pkg --no-scan

# Only scan, don't install
apmw install pkg --scan-only
```

## Action on risk

When a scan detects risk, apmw can take different actions:

```bash
# Abort on risk (default in non-interactive: prompt)
apmw install pkg --on-risk abort

# Proceed despite risk
apmw install pkg --on-risk proceed

# Quarantine the package
apmw install pkg --on-risk quarantine
```

The `--on-risk` flag accepts: `prompt` (default), `abort`, `proceed`,
`quarantine`.

## Supply-chain defense: minimum release age

apmw applies a default 2-day minimum-release-age policy: brand-new releases
(younger than `min_release_age_days`) are skipped to avoid supply-chain
attacks on fresh packages. Configure this in `apmw.toml`:

```toml
min_release_age_days = 2
```

Or via environment variable:

```bash
APMW_MIN_RELEASE_AGE_DAYS=7 apmw install pkg
```
