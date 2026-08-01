# apmw — All Package Manager Wrapper
# Justfile following ADR 20260131001 Standard Developer UX Flow

# _devbox helper — DRY auto-detection of devbox environment
# All normal targets delegate to this. Checks DEVBOX_SHELL_ENABLED and either
# runs the _impl target directly, re-execs via devbox run, or falls back to doctor.
_devbox target *args:
    #!/usr/bin/env bash
    if [ "${DEVBOX_SHELL_ENABLED:-0}" = "1" ]; then
        exec just "{{target}}" {{args}}
    elif command -v devbox >/dev/null 2>&1; then
        exec devbox run -- just "{{target}}" {{args}}
    else
        echo "devbox not found in PATH." >&2
        echo "Running doctor to diagnose environment issues..." >&2
        just doctor 2>/dev/null || true
        exit 1
    fi

# Normal targets — Developer interface (REQUIRED)
# One-liners delegating to _devbox
clean:
    just _devbox clean_impl

dev:
    just _devbox dev_impl

build:
    just _devbox build_impl

test:
    just _devbox test_impl

lint:
    just _devbox lint_impl

format:
    just _devbox format_impl

check:
    just _devbox check_impl

audit:
    just _devbox audit_impl

outdated:
    just _devbox outdated_impl

# Bootstrap recipes (REQUIRED)
bootstrap:
    just _devbox bootstrap_impl

# doctor runs DIRECTLY — it's the fallback when devbox is missing
doctor:
    #!/usr/bin/env bash
    echo "Checking development environment..."
    command -v devbox >/dev/null 2>&1 && echo "devbox: $(devbox version 2>&1 | head -1)" || echo "devbox: NOT FOUND"
    command -v just >/dev/null 2>&1 && echo "just: $(just --version 2>&1)" || echo "just: NOT FOUND"
    command -v cargo >/dev/null 2>&1 && echo "cargo: $(cargo --version 2>&1)" || echo "cargo: NOT FOUND"
    command -v rustc >/dev/null 2>&1 && echo "rustc: $(rustc --version 2>&1)" || echo "rustc: NOT FOUND"
    [ "${DEVBOX_SHELL_ENABLED:-0}" = "1" ] && echo "devbox env: ACTIVE" || echo "devbox env: NOT ACTIVE"

# Implementation targets (REQUIRED) — hidden from just --list via _ prefix
bootstrap_impl:
    cargo build
    echo "Development environment ready!"

build_impl:
    cargo build --release

test_impl:
    cargo test

dev_impl:
    cargo run

lint_impl:
    cargo clippy --all-targets --all-features -- -D warnings

format_impl:
    cargo fmt --all

check_impl:
    cargo check --all-targets --all-features

audit_impl:
    cargo audit

outdated_impl:
    cargo outdated

clean_impl:
    cargo clean

# Validation target — runs all quality gates
validate_impl:
    cargo fmt --all -- --check
    cargo clippy --all-targets --all-features -- -D warnings
    cargo test
    cargo doc --no-deps
    echo "All quality gates passed!"

validate:
    just _devbox validate_impl

# Documentation assets — generate man pages and shell completions.
# These run directly (no devbox wrapper) since they only need cargo.
man:
    cargo run --example gen-assets -- man

completions:
    cargo run --example gen-assets -- completions

assets: man completions

man_impl:
    cargo run --example gen-assets -- man

completions_impl:
    cargo run --example gen-assets -- completions
