# apmw — All Package Manager Wrapper
# Justfile following ADR 20260131001 Standard Developer UX Flow

_log := '
_jv_has() {
  local cat="$1"
  local v="${JUST_LOG:-0}"
  case "$v" in
    1|all) return 0 ;;
    0|"") return 1 ;;
  esac
  v="${v//startend/start,end}"
  echo ",$v," | grep -q ",$cat,"
}
log_info()   { _jv_has info   && echo "$*" || true; }
log_start()  { _jv_has start  && echo "▶ $*" || true; }
log_end()    { _jv_has end    && echo "✔ $*" || true; }
log_status() { _jv_has status && echo "$*" || true; }
log_warn()   { echo "⚠️  $*" >&2; }
log_error()  { echo "❌ $*" >&2; }
log_startend() {
  local msg="$1"; shift
  local rc
  _jv_has start && echo "▶ $msg" || true
  rc=0; "$@" || rc=$?
  _jv_has end && echo "✔ $msg complete" || true
  return $rc
}
'

# _devbox helper — DRY auto-detection of devbox environment
# All normal targets delegate to this. Checks DEVBOX_SHELL_ENABLED and either
# runs the _impl target directly, re-execs via devbox run, or falls back to doctor.
_devbox target *args:
    #!/usr/bin/env bash
    {{_log}}
    if [ "${DEVBOX_SHELL_ENABLED:-0}" = "1" ]; then
        exec just "{{target}}" {{args}}
    elif command -v devbox >/dev/null 2>&1; then
        exec devbox run -- just "{{target}}" {{args}}
    else
        log_error "devbox not found in PATH."
        log_warn "Running doctor to diagnose environment issues..."
        just doctor 2>/dev/null || true
        exit 1
    fi

# Normal targets — Developer interface (REQUIRED)
# One-liners delegating to _devbox
clean:
    @just _devbox clean_impl

dev:
    @just _devbox dev_impl

build:
    @just _devbox build_impl

test:
    @just _devbox test_impl

lint:
    @just _devbox lint_impl

format:
    @just _devbox format_impl

check:
    @just _devbox check_impl

audit:
    @just _devbox audit_impl

outdated:
    @just _devbox outdated_impl

# Bootstrap recipes (REQUIRED)
bootstrap:
    @just _devbox bootstrap_impl

# doctor runs DIRECTLY — it's the fallback when devbox is missing
doctor:
    #!/usr/bin/env bash
    echo "Checking development environment..."
    command -v devbox >/dev/null 2>&1 && echo "devbox: $(devbox version 2>&1 | head -1)" || echo "devbox: NOT FOUND"
    command -v just >/dev/null 2>&1 && echo "just: $(just --version 2>&1)" || echo "just: NOT FOUND"
    command -v cargo >/dev/null 2>&1 && echo "cargo: $(cargo --version 2>&1)" || echo "cargo: NOT FOUND"
    command -v rustc >/dev/null 2>&1 && echo "rustc: $(rustc --version 2>&1)" || echo "rustc: NOT FOUND"
    command -v nono >/dev/null 2>&1 && echo "nono: $(nono --version 2>&1)" || echo "nono: NOT FOUND (install with: brew install nono)"
    [ "${DEVBOX_SHELL_ENABLED:-0}" = "1" ] && echo "devbox env: ACTIVE" || echo "devbox env: NOT ACTIVE"

# Implementation targets (REQUIRED) — hidden from just --list via [private]
[private]
bootstrap_impl:
    #!/usr/bin/env bash
    set -euo pipefail
    {{_log}}
    log_start "Building"
    cargo build
    log_end "Bootstrap complete"

[private]
build_impl:
    cargo build --release

[private]
test_impl:
    cargo test

[private]
dev_impl:
    cargo run

[private]
lint_impl:
    cargo clippy --all-targets --all-features -- -D warnings

[private]
format_impl:
    cargo fmt --all

[private]
check_impl:
    cargo check --all-targets --all-features

[private]
audit_impl:
    cargo audit

[private]
outdated_impl:
    cargo outdated

[private]
clean_impl:
    cargo clean

# Validation target — runs all quality gates
[private]
validate_impl:
    #!/usr/bin/env bash
    set -euo pipefail
    {{_log}}
    log_start "Running quality gates"
    cargo fmt --all -- --check
    cargo clippy --all-targets --all-features -- -D warnings
    cargo test
    cargo doc --no-deps
    cargo audit
    # Non-blocking: cargo outdated defaults to --exit-code 0 (reports only).
    # Warn on failure so a network error doesn't abort the whole validate.
    cargo outdated || log_warn "cargo outdated failed (non-blocking)"
    log_end "All quality gates passed"

validate:
    @just _devbox validate_impl

# Documentation assets — generate man pages and shell completions.
# These run directly (no devbox wrapper) since they only need cargo.
man:
    cargo run --example gen-assets -- man

completions:
    cargo run --example gen-assets -- completions

assets: man completions

[private]
man_impl:
    cargo run --example gen-assets -- man

[private]
completions_impl:
    cargo run --example gen-assets -- completions
