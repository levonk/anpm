#!/usr/bin/env bash
# shellcheck shell=bash
# Shared metadata for apmw packaging generators.
# Source this file from any generator: `source "$(dirname "$0")/../metadata.sh"`
#
# All ecosystem packages wrap the pre-built Rust binary — the binary is the
# single source of truth. Each package downloads the platform-appropriate
# archive from the GitHub release and installs the binary.

set -euo pipefail

# --- Project metadata (mirrors Cargo.toml) -------------------------------
APMW_NAME="apmw"
APMW_VERSION="${APMW_VERSION:-0.1.0}"
APMW_DESCRIPTION="All Package Manager Wrapper — abstracts every package installer into one intelligent surface"
APMW_LICENSE="MIT"
APMW_REPOSITORY="https://github.com/levonk/apmw"
APMW_HOMEPAGE="https://github.com/levonk/apmw"
APMW_AUTHORS="levonk"
APMW_MAINTAINER="levonk <https://github.com/levonk>"

# --- Release asset naming ------------------------------------------------
# Release archives follow the pattern:
#   apmw-${VERSION}-${TARGET}.tar.gz   (unix)
#   apmw-${VERSION}-${TARGET}.zip      (windows)
# Each archive contains a single `apmw` (or `apmw.exe`) binary.

# Map a Rust target triple to a friendly platform slug used in download URLs.
target_to_slug() {
    case "$1" in
        x86_64-unknown-linux-gnu)   echo "x86_64-unknown-linux-gnu" ;;
        aarch64-unknown-linux-gnu)  echo "aarch64-unknown-linux-gnu" ;;
        x86_64-apple-darwin)        echo "x86_64-apple-darwin" ;;
        aarch64-apple-darwin)       echo "aarch64-apple-darwin" ;;
        x86_64-pc-windows-msvc)     echo "x86_64-pc-windows-msvc" ;;
        *)                          echo "$1" ;;
    esac
}

# Return the archive extension for a target (tar.gz for unix, zip for windows).
target_archive_ext() {
    case "$1" in
        *windows*) echo "zip" ;;
        *)         echo "tar.gz" ;;
    esac
}

# Return the binary name for a target (apmw.exe on windows, apmw elsewhere).
target_binary_name() {
    case "$1" in
        *windows*) echo "apmw.exe" ;;
        *)         echo "apmw" ;;
    esac
}

# Construct the download URL for a release asset.
#   download_url <version> <target>
download_url() {
    local version="$1"
    local target="$2"
    local slug
    slug="$(target_to_slug "$target")"
    local ext
    ext="$(target_archive_ext "$target")"
    echo "${APMW_REPOSITORY}/releases/download/v${version}/apmw-${version}-${slug}.${ext}"
}

# Strip a leading 'v' from a version string if present.
strip_v() {
    local v="$1"
    echo "${v#v}"
}

# Print the list of supported Rust target triples, one per line.
supported_targets() {
    cat <<'TARGETS'
x86_64-unknown-linux-gnu
aarch64-unknown-linux-gnu
x86_64-apple-darwin
aarch64-apple-darwin
x86_64-pc-windows-msvc
TARGETS
}
