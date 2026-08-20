#!/usr/bin/env bash
# Devbox package generator for apmw.
#
# Produces a devbox.json that installs the pre-built Rust binary via a
# post-install hook that downloads the platform-appropriate archive from the
# GitHub release.
#
# Usage:   ./generate.sh [version] [output-dir]
# Example: ./generate.sh 0.1.0 ./dist
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../metadata.sh
source "$SCRIPT_DIR/../metadata.sh"

VERSION="$(strip_v "${1:-$APMW_VERSION}")"
OUTPUT_DIR="${2:-$SCRIPT_DIR/dist}"

mkdir -p "$OUTPUT_DIR"

cat > "$OUTPUT_DIR/devbox.json" <<EOF
{
  "packages": [],
  "shell": {
    "init_hook": [
      "export APMW_VERSION=${VERSION}",
      "export APMW_BIN_DIR=\"\${DEVBOX_PROJECT_DIR}/.apmw-bin\"",
      "mkdir -p \"\$APMW_BIN_DIR\"",
      "if [ ! -x \"\$APMW_BIN_DIR/apmw\" ]; then",
      "  case \"\$(uname -s)-\$(uname -m)\" in",
      "    Linux-x86_64)  APMW_TARGET=x86_64-unknown-linux-gnu ;;",
      "    Linux-aarch64) APMW_TARGET=aarch64-unknown-linux-gnu ;;",
      "    Darwin-x86_64) APMW_TARGET=x86_64-apple-darwin ;;",
      "    Darwin-arm64)  APMW_TARGET=aarch64-apple-darwin ;;",
      "    *) echo \"apmw: unsupported platform\" >&2; return 1 ;;",
      "  esac",
      "  curl -fsSL \"${APMW_REPOSITORY}/releases/download/v${VERSION}/apmw-${VERSION}-\${APMW_TARGET}.tar.gz\" | tar -xz -C \"\$APMW_BIN_DIR\"",
      "  chmod +x \"\$APMW_BIN_DIR/apmw\"",
      "fi",
      "export PATH=\"\$APMW_BIN_DIR:\$PATH\""
    ]
  },
  "nixpkgs": {
    "commit": "f807e7b39c19a3a1b2e2fcfb6e3e6c1d18e73a6f"
  }
}
EOF

cat > "$OUTPUT_DIR/README.md" <<EOF
# apmw Devbox package

${APMW_DESCRIPTION}

Drop this \`devbox.json\` into your project (or merge the \`shell.init_hook\`
into your existing devbox.json). On \`devbox shell\` the init hook downloads
the pre-built apmw binary for your platform from the
[GitHub release](${APMW_REPOSITORY}/releases) and places it on PATH.
EOF

echo "Generated Devbox package in $OUTPUT_DIR/devbox.json"
