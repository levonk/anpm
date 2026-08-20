#!/usr/bin/env bash
# Homebrew formula generator for apmw.
#
# Produces a Homebrew formula that downloads the pre-built Rust binary for
# the host platform (macOS x86_64/arm64) from the GitHub release and installs
# it into the Homebrew bin directory.
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

cat > "$OUTPUT_DIR/apmw.rb" <<EOF
# apmw Homebrew formula — wraps the pre-built Rust binary.
class Apmw < Formula
  desc "${APMW_DESCRIPTION}"
  homepage "${APMW_HOMEPAGE}"
  url "${APMW_REPOSITORY}/releases/download/v${VERSION}/apmw-${VERSION}-x86_64-apple-darwin.tar.gz"
  version "${VERSION}"
  sha256 "0000000000000000000000000000000000000000000000000000000000000000"
  license "${APMW_LICENSE}"

  on_macos do
    on_arm do
      url "${APMW_REPOSITORY}/releases/download/v${VERSION}/apmw-${VERSION}-aarch64-apple-darwin.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
  end

  def install
    bin.install "apmw"
  end

  test do
    assert_match "apmw", shell_output("#{bin}/apmw --version", 2)
  end
end
EOF

cat > "$OUTPUT_DIR/README.md" <<EOF
# apmw Homebrew formula

${APMW_DESCRIPTION}

## Install from tap

\`\`\`
brew tap levonk/apmw https://github.com/levonk/homebrew-apmw
brew install apmw
\`\`\`

## Install from formula file

\`\`\`
brew install ./apmw.rb
\`\`\`

**Note:** Replace the \`sha256\` placeholders with the actual checksums from
the [release assets](${APMW_REPOSITORY}/releases) before publishing.
EOF

echo "Generated Homebrew formula in $OUTPUT_DIR/apmw.rb"
